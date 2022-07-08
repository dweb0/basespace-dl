use basespace_dl::util;
use basespace_dl::workspace::Workspace;
use clap::{App, Arg, ArgMatches};
use console::style;
use failure::bail;
use failure::ResultExt;
use futures::future;
use log::{info, warn};
use regex::Regex;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use tabwriter::TabWriter;

fn build_app() -> App<'static, 'static> {
    App::new("basespace-dl")
        .version(env!("CARGO_PKG_VERSION"))
        .author("dweb0")
        .about("Multi-account basespace file downloader")
        .args(&[
            Arg::with_name("project")
                .index(1)
                .required(true)
                .takes_value(true)
                .help("Project name (e.g. project17890). Use ALL to print all projects"),
            Arg::with_name("list-files")
                .long("list-files")
                .short("F")
                .takes_value(false)
                .help("List all files for a given project. Use -l to also print file metadata."),
            Arg::with_name("pattern")
                .long("pattern")
                .short("p")
                .takes_value(true)
                .help("Only select files according to this regex pattern"),
            Arg::with_name("select-files")
                .long("select-files")
                .short("f")
                .takes_value(true)
                .help("Only select files from this list. Accepts a file or - for STDIN."),
            Arg::with_name("directory")
                .long("directory")
                .short("d")
                .takes_value(true)
                .help("Download files to this directory."),
            Arg::with_name("undetermined")
                .long("undetermined")
                .short("U")
                .required(false)
                .takes_value(false)
                .help("Fetch undetermined files as well. These are stored in the \"Unindexed Reads\" project."),
            Arg::with_name("config")
                .long("config")
                .short("C")
                .takes_value(true)
                .help("Alternate config. Stored in $HOME/.config/basespace-dl/{name}.toml"),
            Arg::with_name("skip-completion-check")
                .long("skip-completion-check")
                .required(false)
                .takes_value(false)
                .help("Skip the requirement that all samples in a project be finished processing"),
            Arg::with_name("long-format")
                .long("long-format")
                .short("l")
                .takes_value(false)
                .help("Long format. Prints file size if listing files or more project info if listing projects"),
            Arg::with_name("verbose")
                .long("verbose")
                .short("v")
                .required(false)
                .help("Print status messages"),
        ])
}

#[tokio::main]
async fn main() {
    let app = build_app();
    let matches = app.get_matches();

    if matches.is_present("verbose") {
        std::env::set_var("RUST_LOG", "info");
    } else {
        std::env::set_var("RUST_LOG", "warn");
    }
    env_logger::init();

    if let Err(e) = real_main(matches).await {
        eprintln!("{} {}", style("error:").bold().red(), e);
        std::process::exit(1);
    }
}

async fn real_main(matches: ArgMatches<'static>) -> Result<(), failure::Error> {
    let ws = match matches.value_of("config") {
        Some(config) => Workspace::with_config(config),
        None => Workspace::new(),
    }
    .with_context(|e| format!("Could not generate workspace. {}", e))?;

    let directory = match matches.value_of("directory") {
        Some(dir) => {
            let path_dir = PathBuf::from(dir);
            if !path_dir.is_dir() {
                bail!("{} is not a valid directory", dir);
            }
            path_dir
        }
        None => PathBuf::from("."),
    };

    let multi = ws
        .to_multiapi()
        .with_context(|e| format!("Could not generate multi-api from workspace. {}", e))?;
    let query = matches.value_of("project").unwrap();
    let projects = multi.get_projects().await?;

    if query == "ALL" {
        if matches.is_present("long-format") {
            
            // Only print YYYY-MM-DD and not timestamp
            // Timestamp should ALWAYS include this
            let date_re = Regex::new(r"^\d{4,4}-\d{2,2}-\d{2,2}").unwrap();

            for project in projects {
                let date = match date_re.find(&project.date_created) {
                    Some(mat) => mat.as_str(),
                    None => ""
                };
                
                println!(
                    "{},{},{},{}",
                    project.name,
                    project.user_owned_by.id,
                    project.user_owned_by.name,
                    date,
                );
            }
        } else {
            for project in projects {
                println!("{}", project.name);
            }
        }
        std::process::exit(0);
    }

    let mut matching_projects: Vec<_> = projects.iter().filter(|p| p.name == query).collect();
    let project = if matching_projects.is_empty() {
        let candidates = util::did_you_mean(query, projects);
        if candidates.is_empty() {
            bail!("no such project {}.", query);
        }
        bail!(
            "no such project {}. Did you mean one of these?\n\n{}",
            query,
            candidates.join("\n")
        );
    } else if matching_projects.len() > 1 {
        util::resolve_duplicate_projects(matching_projects)
    } else {
        matching_projects.remove(0)
    };

    let samples = if matches.is_present("undetermined") {
        if project.user_fetched_by_id != project.user_owned_by.id {
            bail!("Must be the owner of a project to access its \"Unindexed Reads\".");
        }
        let unindexed_reads = projects.iter().find(|x| {
            x.name == "Unindexed Reads" && x.user_fetched_by_id == project.user_fetched_by_id
        });

        let unindexed_reads = match unindexed_reads {
            Some(unindexed_reads) => unindexed_reads,
            None => bail!("Could not find Unindexed Reads in basespace account."),
        };

        let undetermined_sample = multi.get_undetermined_sample(project, unindexed_reads);
        let samples = multi.get_samples(project);

        // Fetch main samples + the undetermined sample concurrently
        let (samples, undetermined_sample) = future::join(samples, undetermined_sample).await;
        let mut samples = samples?;

        let undetermined_sample = undetermined_sample?;
        samples.push(undetermined_sample);
        samples
    } else {
        multi.get_samples(project).await?
    };

    let (samples, unfinished_samples): (Vec<_>, Vec<_>) =
        samples.into_iter().partition(|s| s.status == "Complete");

    if !unfinished_samples.is_empty() {
        if matches.is_present("skip-completion-check") {
            warn!(
                "Warning: {} samples still processing. Downloading anyway.",
                unfinished_samples.len()
            );
        } else {
            bail!(
                "Project not finished yet. {} samples still processing.",
                unfinished_samples.len()
            );
        }
    }

    info!("Found {} completed samples", samples.len());
    info!("Fetching files...");

    let mut files = multi.get_files(project, &samples).await?;
    if let Some(pattern) = matches.value_of("pattern") {
        let re = Regex::new(pattern).with_context(|e| format!("Invalid regex pattern. {}", e))?;
        files = files
            .into_iter()
            .filter(|file| re.find(&file.name).is_some())
            .collect();
    }

    if let Some(filelist) = matches.value_of("select-files") {
        let mut rdr: Box<dyn Read> = match filelist {
            "-" => Box::new(std::io::stdin()),
            _ => Box::new(File::open(filelist)?),
        };
        let mut buffer = String::new();
        rdr.read_to_string(&mut buffer)?;
        let filter_list: HashSet<String> = buffer.lines().map(|line| line.to_owned()).collect();
        files = files
            .into_iter()
            .filter(|file| filter_list.contains(&file.name))
            .collect();
    }

    if matches.is_present("list-files") {
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();

        if matches.occurrences_of("list-files") > 1 || matches.is_present("long-format") {
            let mut writer = TabWriter::new(&mut stdout);
            for file in files {
                writeln!(
                    &mut writer,
                    "{:>4}\t{}",
                    util::convert_bytes(file.size as f64),
                    file.name
                )
                .unwrap_or_else(|_| {
                    std::process::exit(0);
                });
            }
            writer.flush()?;
        } else {
            for file in files {
                writeln!(&mut stdout, "{}", file.name).unwrap_or_else(|_| {
                    std::process::exit(0);
                });
            }
        }
        std::process::exit(0);
    }

    info!("Downloading {} files...", files.len());

    multi
        .download_files(&files, project, directory)
        .with_context(|e| format!("Could not download files. {}", e))?;

    Ok(())
}

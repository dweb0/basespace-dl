use basespace_dl::{self, workspace::Workspace, write_err};
use clap::{App, Arg};
use regex::Regex;
use std::cmp::Reverse;
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use log::info;

fn main() {
    let matches = App::new("basespace-dl")
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
                .help("List all files for a given project"),
            Arg::with_name("pattern")
                .long("pattern")
                .short("p")
                .takes_value(true)
                .help("Only select files according to this regex pattern"),
            Arg::with_name("select-files")
                .long("select-files")
                .short("f")
                .takes_value(true)
                .help("Only select files from this list. Use - for STDIN."),
            Arg::with_name("directory")
                .long("directory")
                .short("d")
                .takes_value(true)
                .help("Download files to this directory."),
            Arg::with_name("sort-by")
                .long("sort-by")
                .short("s")
                .required(false)
                .takes_value(true)
                .possible_values(&["name", "size-smallest", "size-biggest"])
                .help("Sort files by attribute"),
            Arg::with_name("undetermined")
                .long("undetermined")
                .short("U")
                .required(false)
                .takes_value(false)
                .help("Fetch undetermined files as well"),
            Arg::with_name("config")
                .long("config")
                .short("C")
                .takes_value(true)
                .help("Alternate config. Stored in $HOME/.config/basespace-dl/{name}.toml"),
            Arg::with_name("verbose")
                .long("verbose")
                .short("v")
                .required(false)
                .help("Print status messages")
        ])
        .get_matches();

    if matches.is_present("verbose") {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let ws = match matches.value_of("config") {
        Some(config) => Workspace::with_config(config),
        None => Workspace::new()
    };

    let ws = match ws {
        Ok(ws) => ws,
        Err(e) => {
            write_err(&format!("Could not generate workspace. {}", e));
        }
    };

    let multi = match ws.to_multiapi() {
        Ok(multi) => multi,
        Err(e) => {
            write_err(&format!(
                "Could not generate multi-api from workspace. {}",
                e
            ));
        }
    };

    let directory = match matches.value_of("directory") {
        Some(dir) => {
            let path_dir = PathBuf::from(dir);
            if !path_dir.is_dir() {
                write_err(&format!("{} is not a valid directory", dir));
            }
            path_dir
        }
        None => PathBuf::from("."),
    };

    let query = matches.value_of("project").unwrap();
    let print_all = match query {
        "ALL" => true,
        _ => false,
    };

    info!("Searching for project...");

    let project = multi.find_project(matches.value_of("project").unwrap(), print_all);

    let project = match project {
        Some(project) => project,
        None => {
            if print_all {
                std::process::exit(0);
            }
            write_err(&format!("Could not find project {}", query));
        }
    };

    info!("Fetching samples...");
    
    let mut samples = match multi.get_samples_by_project(&project) {
        Ok(samples) => samples,
        Err(e) => {
            write_err(e);
        }
    };

    if matches.is_present("undetermined") {
        let undetermined_sample = match multi.get_undetermined_sample(&project) {
            Ok(sample) => sample,
            Err(e) => {
                write_err(&format!("Could not get undetermined sample. {}", e));
            }
        };
        samples.push(undetermined_sample);
    }

    info!("Locating files...");    

    let mut files = match multi.get_files_from_samples(samples, &project) {
        Ok(files) => files,
        Err(e) => {
            write_err(e);
        }
    };

    if let Some(pattern) = matches.value_of("pattern") {
        let re = match Regex::new(pattern) {
            Ok(re) => re,
            Err(e) => {
                write_err(&format!("Invalid regex pattern. {}", e));
            }
        };
        files = files
            .into_iter()
            .filter(|file| re.find(&file.name).is_some())
            .collect();
    }

    if let Some(filelist) = matches.value_of("select-files") {
        let mut reader: Box<dyn Read> = match filelist {
            "-" => {
                Box::new(std::io::stdin())
            },
            _ => {
                let file = match File::open(filelist) {
                    Ok(file) => file,
                    Err(e) => {
                        write_err(e);
                    }
                };
                Box::new(file)
            }
        };
        let mut buffer = String::new();
        reader.read_to_string(&mut buffer).unwrap();
        let filter_list: HashSet<String> = buffer.lines().map(|line| line.to_owned()).collect();
        files = files
            .into_iter()
            .filter(|file| filter_list.contains(&file.name))
            .collect();
    }

    if let Some(v) = matches.value_of("sort-by") {
        match v {
            "name" => {
                files.sort_unstable_by_key(|file| file.name.to_owned());
            }
            "size-smallest" => {
                files.sort_unstable_by_key(|file| file.size);
            }
            "size-biggest" => {
                files.sort_unstable_by_key(|file| Reverse(file.size));
            }
            _ => unreachable!(),
        };
    }
    if matches.is_present("list-files") {
        for file in files {
            println!("{}", file.name);
        }
    } else {
        info!("Downloading files...");
        match multi.download_files(files, &project, directory) {
            Ok(_) => (),
            Err(e) => {
                write_err(&format!("Could not download files. {}", e));
            }
        }
    }
}

use basespace_dl::{self, api::Sample, workspace::Workspace};
use clap::{App, Arg};
use regex::Regex;

use std::cmp::Reverse;
use std::path::PathBuf;
use exitfailure::ExitFailure;

fn main() -> Result<(), ExitFailure> {
    let matches = App::new("basespace-dl")
        .version("0.1.1")
        .author("dweb0")
        .about("Download files from multiple basespace accounts.")
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
                .help("Fetch undetermined files (and download if desired)")
        ])
        .get_matches();

    let multi = Workspace::new()?.to_multiapi()?;

    let directory = match matches.value_of("directory") {
        Some(dir) => {
            let path_dir = PathBuf::from(dir);
            if !path_dir.is_dir() {
                eprintln!("Error: {} is not a valid directory.", dir);
                std::process::exit(1);
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

    let project = multi.find_project(matches.value_of("project").unwrap(), print_all);
    match project {
        Some(project) => {
            let mut samples = multi.get_samples_by_project(&project)?;
            if let Some(pattern) = matches.value_of("pattern") {
                let re = Regex::new(pattern)?;
                filter_samples(&mut samples, &re);
            }
            if matches.is_present("undetermined") {
                let undetermined_sample = multi.get_undetermined_sample(&project)?;
                samples.push(undetermined_sample);
            }
            let mut files = multi.get_files_from_samples(samples, &project)?;
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
                    _ => panic!("Invalid sort-by arg. This should be unreachable"),
                };
            }
            if matches.is_present("list-files") {
                for file in files {
                    println!("{}", file.name);
                }
            } else {
                multi.download_files(files, &project, directory)?;
            }
        }
        None => {
            if !print_all {
                eprintln!("Error: Could not find project {}", query);
            }
        }
    }

    Ok(())
}

// TODO: Replace with drain_filter when it becomes stable
fn filter_samples(samples: &mut Vec<Sample>, re: &Regex) {

    let mut i = 0;
    while i != samples.len() {
        let sample = &mut samples[i];
        if re.find(&sample.name).is_none() {
            samples.remove(i);
        } else {
            i += 1;
        }
    }
}

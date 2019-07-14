use basespace_dl::{self, api::Sample, workspace::Workspace};
use clap::{App, Arg};
use console::style;
use regex::Regex;
use std::cmp::Reverse;
use std::path::PathBuf;

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
        ])
        .get_matches();

    let ws = match Workspace::new() {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!(
                "{} Could not generate workspace. {}",
                style(&format!("error:")).bold().red(),
                e
            );
            std::process::exit(1);
        }
    };

    let multi = match ws.to_multiapi() {
        Ok(multi) => multi,
        Err(e) => {
            eprintln!(
                "{} Could not generate multi-api from workspace. {}",
                style(&format!("error:")).bold().red(),
                e
            );
            std::process::exit(1);
        }
    };

    let directory = match matches.value_of("directory") {
        Some(dir) => {
            let path_dir = PathBuf::from(dir);
            if !path_dir.is_dir() {
                eprintln!(
                    "{} {} is not a valid directory.",
                    style(&format!("error:")).bold().red(),
                    dir
                );
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

    let project = match project {
        Some(project) => project,
        None => {
            if print_all {
                std::process::exit(0);
            }
            eprintln!(
                "{} Could not find project {}.",
                style(&format!("error:")).bold().red(),
                query
            );
            std::process::exit(1);
        }
    };

    let mut samples = match multi.get_samples_by_project(&project) {
        Ok(samples) => samples,
        Err(e) => {
            eprintln!("{} {}", style(&format!("error:")).bold().red(), e);
            std::process::exit(1);
        }
    };

    if let Some(pattern) = matches.value_of("pattern") {
        let re = match Regex::new(pattern) {
            Ok(re) => re,
            Err(e) => {
                eprintln!(
                    "{} Invalid regex pattern. {}",
                    style(&format!("error:")).bold().red(),
                    e
                );
                std::process::exit(1);
            }
        };
        filter_samples(&mut samples, &re);
    }
    if matches.is_present("undetermined") {
        let undetermined_sample = match multi.get_undetermined_sample(&project) {
            Ok(sample) => sample,
            Err(e) => {
                eprintln!(
                    "{} Could not get undetermined sample. {}",
                    style(&format!("error:")).bold().red(),
                    e
                );
                std::process::exit(1);
            }
        };
        samples.push(undetermined_sample);
    }

    let mut files = match multi.get_files_from_samples(samples, &project) {
        Ok(files) => files,
        Err(e) => {
            eprintln!("{} {}", style(&format!("error:")).bold().red(), e);
            std::process::exit(1);
        }
    };

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
        match multi.download_files(files, &project, directory) {
            Ok(_) => (),
            Err(e) => {
                eprintln!(
                    "{} Could not download files. {}",
                    style(&format!("error:")).bold().red(),
                    e
                );
                std::process::exit(1);
            }
        }
    }
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

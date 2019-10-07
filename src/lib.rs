#[macro_use]
extern crate text_io;

use console::style;
use crossbeam_channel::unbounded;
use failure::bail;
use futures::{stream, Future, Stream};
use pretty_bytes::converter::convert as convert_bytes;
use rayon::prelude::*;
use reqwest::header::HeaderValue;
use reqwest::r#async::Client;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;
use log::warn;
use indicatif::ProgressBar;
use std::fs;

pub mod api;
pub mod workspace;
use api::*;

pub static RESPONSE_LIMIT: &str = "1024";
pub static BASESPACE_URL: &str = "https://api.basespace.illumina.com/v1pre3";

pub struct MultiApi {
    pub accounts: HashMap<String, String>,
}

impl MultiApi {
    pub fn new(accounts: HashMap<String, String>) -> MultiApi {
        MultiApi { accounts }
    }

    fn get_token(&self, project: &Project) -> String {
        self.accounts
            .get(&project.user_fetched_by_id)
            .expect("Could not get token from accounts")
            .to_owned()
    }

    pub fn get_samples_by_project(&self, project: &Project) -> Result<Vec<Sample>, failure::Error> {

        let token = self.get_token(&project);
        let client = reqwest::Client::new();

        let response: SampleResponse = client
            .get(&format!(
                "{}/projects/{}/samples?limit={}",
                BASESPACE_URL, project.id, RESPONSE_LIMIT
            ))
            .header("x-access-token", token)
            .send()?
            .json()?;

        // Make sure that all samples are finished before downloading
        let (finished, unfinished): (Vec<Sample>, Vec<Sample>) = response
            .samples()
            .into_iter()
            .partition(|s| s.status == "Complete");

        if !unfinished.is_empty() {
            bail!(
                "Project not finished yet. {} samples still processing",
                unfinished.len()
            );
        }

        Ok(finished)
    }

    pub fn download_files<D>(
        &self,
        files: Vec<DataFile>,
        project: &Project,
        dir: D,
    ) -> Result<(), failure::Error>
    where
        D: AsRef<Path>,
    {

        if files.is_empty() {
            bail!("Selected 0 files.");
        }
        let token = self.get_token(&project);
        let dir = dir.as_ref();

        let num_files = files.len();
        let total_size: i64 = files.iter().map(|file| file.size).sum();

        // TODO: Use AtomicUsize instead of Mutex
        let index: Mutex<usize> = Mutex::new(0);
        let time_before = Instant::now();

        let pb = ProgressBar::new(files.len() as u64);
 
        let _: Vec<Result<(), failure::Error>> = files
            .par_iter()
            .map(|file| {
                {
                    let mut index = index.lock().unwrap();
                    *index += 1;
                    pb.println(
                        &format!(
                            "{} {} : {}",
                            style(&format!("[{}/{}]", index, num_files)).bold().dim(),
                            &file.name,
                            convert_bytes(file.size as f64)
                        )
                    );
                }

                let client = reqwest::Client::new();
                let mut resp = client
                    .get(&format!("{}/files/{}/content", BASESPACE_URL, file.id))
                    .header("x-access-token", HeaderValue::from_str(&token)?)
                    .send()?;

                let joined_path = dir.join(&file.name);

                // Need separate scope since we need to close
                // the file before calculating the etag.
                {
                    let mut file = File::create(&joined_path)?;
                    let mut writer = BufWriter::new(&mut file);

                    loop {
                        let mut buffer = vec![0; 1024];
                        let bcount = resp.read(&mut buffer[..]).unwrap();
                        buffer.truncate(bcount);
                        if !buffer.is_empty() {
                            writer.write_all(&buffer)?;
                        } else {
                            pb.inc(1);
                            break;
                        }
                    }
                }

                // TODO: It's a better idea to use etag to ensure file integrity,
                // but this requires knowing the etag part size
                if fs::metadata(&joined_path)?.len() != file.size as u64 {
                    bail!("{} did not match expected file size.", file.name);
                }

                Ok(())
            })
            .inspect(|result| {
                if let Err(e) = result {
                    eprintln!("{} {}", style("warning:").bold().yellow(), e);
                }
            })
            .filter(|result| result.is_err())
            .collect();

        pb.finish_and_clear();

        let elapsed = time_before.elapsed().as_millis();

        // Cannot divide by 0 or panic!
        if elapsed > 0 {
            // Converting ms back to seconds
            let speed = ((total_size as f64) / (elapsed as f64)) * 1000.0; 
            eprintln!(
                "{} Downloaded {} files at {}/s",
                style("success:").bold().green(),
                num_files,
                convert_bytes(speed)
            );
        }
        
        Ok(())
    }

    pub fn get_files_from_samples(
        &self,
        samples: Vec<Sample>,
        project: &Project,
    ) -> Result<Vec<DataFile>, failure::Error> {
        if samples.is_empty() {
            bail!("No samples for {}", project.name);
        }

        let token = self.get_token(&project);
        let client = Client::new();
        let (tx, rx) = unbounded::<DataFile>();

        let catcher = std::thread::spawn(move || {
            let mut files = vec![];
            while let Ok(file) = rx.recv() {
                files.push(file);
            }
            files
        });

        // Could use par_iter instead, which would reduce binary size as 
        // we no longer need tokio, but tokio seems to be consistently
        // faster here
        let sample_len = samples.len();
        let bodies = stream::iter_ok(samples)
            .map(move |sample| {
                client
                    .get(&format!("{}/samples/{}/files", BASESPACE_URL, sample.id))
                    .header("x-access-token", token.to_owned())
                    .send()
                    .and_then(|res| res.into_body().concat2().from_err())
            })
            .buffer_unordered(sample_len);

        let work = bodies
            .for_each(move |b| {
                let resp: FileResponse = serde_json::from_slice(&b).unwrap();
                for file in resp.files() {
                    tx.send(file).unwrap();
                }
                Ok(())
            })
            .map_err(|_e| panic!("Error"));

        tokio::run(work);
        Ok(catcher.join().unwrap())
    }

    pub fn find_project(&self, query_project: &str, print_all: bool) -> Option<Project> {
        let (tx, rx) = unbounded::<Project>();

        let query_project = query_project.to_owned();
        let catcher = std::thread::spawn(move || {
            let mut found_projects = vec![];
            while let Ok(project) = rx.recv() {
                if print_all {
                    println!("{}", project.name);
                } else if project.name == query_project {
                    found_projects.push(project);
                }
            }
            if found_projects.len() == 1 {
                Some(found_projects.remove(0))
            } else if found_projects.is_empty() {
                None
            } else {
                eprintln!(
                    "{} Found {} projects with the same name.",
                    style("warning:").bold().yellow(),
                    found_projects.len()
                );

                for (index, project) in found_projects.iter().enumerate() {
                    eprintln!(
                        "[{}] name = \"{}\", dateCreated = \"{}\"",
                        index, project.user_owned_by.name, project.date_created
                    );
                }

                let user_index = loop {
                    eprint!(
                        "Enter the project index [0..{}]: ",
                        found_projects.len() - 1
                    );
                    let response: Result<usize, _> = try_read!();
                    let response = match response {
                        Ok(response) => {
                            if response > found_projects.len() - 1 {
                                eprintln!(
                                    "{} Please enter an integer from 0 to {}",
                                    style("error:").bold().red(),
                                    found_projects.len() - 1
                                );
                                continue;
                            }
                            response
                        }
                        Err(_) => {
                            eprintln!(
                                "{} Please enter an integer from 0 to {}",
                                style("error:").bold().red(),
                                found_projects.len() - 1
                            );
                            continue;
                        }
                    };
                    break response;
                };
                Some(found_projects.remove(user_index))
            }
        });

        // Using rayon here instead of tokio because we need
        // to keep track of the account id used to make the request.
        // This is used to fetch shared projects.
        let _: Vec<Result<(), failure::Error>> = self.accounts
            .par_iter()
            .map(|(id, token)| {

                let client = reqwest::Client::new();
                let response: ProjectResponse = client
                    .get(&format!(
                        "{}/users/current/projects?limit={}",
                        BASESPACE_URL, RESPONSE_LIMIT
                    ))
                    .header("x-access-token", HeaderValue::from_str(token)?)
                    .send()?
                    .json()?;
                for project in response.projects_as_user(id) {
                    tx.send(project).expect("Could not send project to sink");
                }
                Ok(())
            })
            .inspect(|result| {
                if let Err(e) = result {
                    warn!("{}", e);
                }
            })
            .collect();
        drop(tx);
        catcher.join().unwrap()
    }

    pub fn get_undetermined_sample(&self, project: &Project) -> Result<Sample, failure::Error> {

        if project.user_fetched_by_id != project.user_owned_by.id {
            bail!("Must be the owner of a project to access its \"Unindexed Reads\".");
        }

        let token = self.get_token(&project);
        let client = reqwest::Client::new();

        let resp: ProjectResponse = client
            .get(&format!(
                "{}/users/current/projects?limit={}",
                BASESPACE_URL, RESPONSE_LIMIT
            ))
            .header("x-access-token", HeaderValue::from_str(&token)?)
            .send()?
            .json()?;

        let projects = resp.projects_as_user(&project.user_owned_by.id);
        let unindexed_project = projects.into_iter().find(|x| x.name == "Unindexed Reads");
        let unindexed_project = match unindexed_project {
            Some(p) => p,
            None => {
                bail!("Could not find Unindexed Reads in basespace account");
            }
        };

        let samples = self.get_samples_by_project(&unindexed_project)?;
        let undetermined_sample = samples.into_iter().find(|x| match &x.experiment_name {
            Some(experiment_name) => experiment_name == &project.name,
            None => false
        });
        match undetermined_sample {
            Some(s) => Ok(s),
            None => {
                bail!(
                    "Could not find Undetermined sample for project {}",
                    &project.name
                );
            }
        }
    }
}

pub fn get_user_from_token(token: &str) -> Result<String, failure::Error> {
    let client = reqwest::Client::new();

    let response: CurrentUserResponse = client
        .get(&format!("{}/users/current", BASESPACE_URL))
        .header("x-access-token", token)
        .send()?
        .json()?;

    let user_id = response.user_id().to_owned();
    Ok(user_id)
}

/// Write error message and exit
///
/// Helper function to reduce code bloat
#[inline]
pub fn write_err<D: std::fmt::Display>(msg: D) -> ! {
    eprintln!("{} {}", style("error:").bold().red(), msg);
    std::process::exit(1);
}

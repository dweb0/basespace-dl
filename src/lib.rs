#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate text_io;

use crossbeam_channel::unbounded;
use futures::{stream, Future, Stream};
use pretty_bytes::converter::convert as convert_bytes;

use console::style;
use indicatif::ProgressBar;
use reqwest::header::HeaderValue;
use reqwest::r#async::Client;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::io::{Read, Write};
use std::path::Path;

pub mod api;
pub mod workspace;

use api::*;

pub static RESPONSE_LIMIT: &'static str = "1024";
pub static BASESPACE_URL: &'static str = "https://api.basespace.illumina.com/v1pre3";

pub struct MultiApi {
    pub accounts: HashMap<String, String>,
}

impl MultiApi {
    pub fn new(accounts: HashMap<String, String>) -> MultiApi {
        MultiApi { accounts }
    }

    fn get_token(&self, project: &Project) -> String {
        self.accounts
            .get(&project.user_owned_by.id)
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
        let token = self.get_token(&project);
        let dir = dir.as_ref();

        let num_files = files.len();
        for (index, file) in files.into_iter().enumerate() {
            eprintln!(
                "{} {} : {}",
                style(&format!("[{}/{}]", index + 1, num_files))
                    .bold()
                    .dim(),
                &file.name,
                convert_bytes(file.size as f64)
            );

            let pb = ProgressBar::new(file.size as u64);
            let client = reqwest::Client::new();
            let mut resp = client
                .get(&format!("{}/files/{}/content", BASESPACE_URL, file.id))
                .header("x-access-token", HeaderValue::from_str(&token)?)
                .send()
                .unwrap();

            let joined_path = dir.join(file.name);
            let mut file = File::create(joined_path)?;
            let mut writer = BufWriter::new(&mut file);

            loop {
                let mut buffer = vec![0; 1024];
                let bcount = resp.read(&mut buffer[..]).unwrap();
                buffer.truncate(bcount);
                if !buffer.is_empty() {
                    writer.write_all(&buffer)?;
                    pb.inc(bcount as u64);
                } else {
                    break;
                }
            }
            pb.finish_and_clear();
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
        let client = Client::new();

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
            } else if found_projects.len() == 0 {
                None
            } else {
                eprintln!(
                    "{} Found {} projects with the same name.",
                    style(&format!("warning:")).bold().yellow(),
                    found_projects.len()
                );

                for (index, project) in found_projects.iter().enumerate() {
                    eprintln!(
                        "{}: name = \"{}\", dateCreated = \"{}\"",
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
                                    style(&format!("error:")).bold().red(),
                                    found_projects.len() - 1
                                );
                                continue;
                            }
                            response
                        }
                        Err(_) => {
                            eprintln!(
                                "{} Please enter an integer from 0 to {}",
                                style(&format!("error:")).bold().red(),
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

        let tokens: Vec<String> = self.accounts.iter().map(|(_k, v)| v.to_owned()).collect();
        let token_len = tokens.len();
        let bodies = stream::iter_ok(tokens)
            .map(move |token| {
                client
                    .get(&format!(
                        "{}/users/current/projects?limit={}",
                        BASESPACE_URL, RESPONSE_LIMIT
                    ))
                    .header("x-access-token", token)
                    .send()
                    .and_then(|res| res.into_body().concat2().from_err())
            })
            .buffer_unordered(token_len);

        let work = bodies
            .for_each(move |b| {
                let resp: ProjectResponse =
                    serde_json::from_slice(&b).expect("Error deserializing project response");
                for project in resp.projects() {
                    tx.send(project).expect("Could not send project to sink");
                }
                Ok(())
            })
            .map_err(|_e| panic!("Error"));

        tokio::run(work);
        catcher.join().unwrap()
    }

    pub fn get_undetermined_sample(&self, project: &Project) -> Result<Sample, failure::Error> {
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

        let projects = resp.projects();
        let unindexed_project = projects.into_iter().find(|x| x.name == "Unindexed Reads");
        let unindexed_project = match unindexed_project {
            Some(p) => p,
            None => {
                bail!("Could not find Unindexed Reads in basespace account");
            }
        };

        let samples = self.get_samples_by_project(&unindexed_project)?;
        let undetermined_sample = samples.into_iter().find(|x| match &x.experiment_name {
            Some(experiment_name) => {
                if experiment_name == &project.name {
                    return true;
                } else {
                    return false;
                }
            }
            None => {
                return false;
            }
        });
        match undetermined_sample {
            Some(s) => return Ok(s),
            None => {
                bail!(
                    "Could not find Undetermined sample for project {}",
                    &project.name
                );
            }
        };
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

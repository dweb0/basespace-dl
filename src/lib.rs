#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure;

use crossbeam_channel::unbounded;
use futures::{stream, Future, Stream};
use pretty_bytes::converter::convert as convert_bytes;

use std::collections::HashMap;
use reqwest::header::HeaderValue;
use reqwest::r#async::Client;
use std::fs::File;
use std::io::BufWriter;
use std::io::{Read, Write};
use std::path::Path;
use indicatif::ProgressBar;
use console::style;

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

        ensure!(
            unfinished.is_empty(),
            format_err!(
                "Project not finished yet. {} samples still processing",
                unfinished.len()
            )
        );

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
                style(&format!("[{}/{}]", index + 1, num_files)).bold().dim(),
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
        let token = self.get_token(&project);

        let client = Client::new();
        let (tx, rx) = unbounded();

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

        let mut files = vec![];
        while let Ok(file) = rx.recv() {
            files.push(file);
        }

        Ok(files)
    }

    pub fn find_project(&self, query_project: &str, print_all: bool) -> Option<Project> {
        let (tx, rx) = unbounded();
        let client = Client::new();

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

        while let Ok(project) = rx.recv() {
            if print_all {
                println!("{}", project.name);
            } else if project.name == query_project {
                return Some(project);
            }
        }

        None
    }
}


pub fn get_user_from_token(token: &str) -> Result<String, failure::Error> {

    let client = reqwest::Client::new();

    let response: CurrentUserResponse = client
        .get(&format!(
            "{}/users/current",
            BASESPACE_URL
        ))
        .header("x-access-token", token)
        .send()?
        .json()?;

    let user_id = response.user_id().to_owned();
    Ok(user_id)
}
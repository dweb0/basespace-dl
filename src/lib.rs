#[macro_use]
extern crate serde_struct_wrapper;

#[macro_use]
extern crate text_io;

use futures::prelude::*;
use futures::stream::futures_unordered::FuturesUnordered;
use std::collections::HashMap;
use pretty_bytes::converter::convert;
use std::time::Instant;

pub mod api;
pub mod util;
pub mod workspace;
use api::*;
use log::info;
use std::path::Path;
use failure::bail;
use rayon::prelude::*;
use std::fs::File;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::io::{BufWriter, Read, Write};
use console::style;
use indicatif::ProgressBar;
use std::fs;

pub static RESPONSE_LIMIT: &str = "1024";
pub static BASESPACE_URL: &str = "https://api.basespace.illumina.com/v1pre3";

pub struct MultiApi {
    pub accounts: HashMap<String, String>,
}

impl MultiApi {
    pub fn new(accounts: HashMap<String, String>) -> MultiApi {
        MultiApi { accounts }
    }

    pub async fn get_projects(&self) -> Result<Vec<Project>, failure::Error> {
        let client = reqwest::Client::new();
        let mut futures = FuturesUnordered::new();

        for (account_id, token) in &self.accounts {
            info!("Fetching projects for account {}", account_id);
            let account_id = account_id.to_owned();
            let resp = client
                .get(&format!(
                    "{}/users/current/projects?limit={}",
                    BASESPACE_URL, RESPONSE_LIMIT
                ))
                .header("x-access-token", token)
                .send()
                .and_then(|x| x.json::<ProjectResponse>())
                .map(|x| (account_id, x));
            futures.push(resp);
        }

        let mut projects = vec![];
        while let Some((account_id, response)) = futures.next().await {
            if let Ok(response) = response {
                projects.extend(response.projects_as_user(&account_id));
            }
        }

        Ok(projects)
    }

    pub async fn get_files(
        &self,
        project: &Project,
        unindexed_reads: Option<&Project>,
    ) -> Result<Vec<DataFile>, failure::Error> {
        let token = self
            .accounts
            .get(&project.user_fetched_by_id)
            .expect("Could not get token from accounts");

        let client = reqwest::Client::new();

        let mut file_futures = FuturesUnordered::new();

        if let Some(unindexed_reads) = unindexed_reads {
            info!("Fetching undetermined files...");
            let resp = client
                .get(&format!(
                    "{}/projects/{}/samples?limit={}",
                    BASESPACE_URL, unindexed_reads.id, RESPONSE_LIMIT
                ))
                .header("x-access-token", token)
                .send()
                .await?
                .json::<SampleResponse>()
                .await?;
            let mut undetermined_samples: Vec<_> = resp.items.iter().filter(|x| match &x.experiment_name {
                Some(experiment_name) => experiment_name == &project.name,
                None => false
            }).collect();
            let undetermined_sample = if undetermined_samples.is_empty() {
                bail!("Could not find Undetermined sample for project {}", project.name);
            } else if undetermined_samples.len() > 1 {
                util::resolve_duplicate_unindexed_reads(undetermined_samples)
            } else {
                undetermined_samples.remove(0)
            };
            let resp = client
                .get(&format!("{}/samples/{}/files", BASESPACE_URL, undetermined_sample.id))
                .header("x-access-token", token)
                .send();
            file_futures.push(resp);
        }

        let resp = client
            .get(&format!(
                "{}/projects/{}/samples?limit={}",
                BASESPACE_URL, project.id, RESPONSE_LIMIT
            ))
            .header("x-access-token", token)
            .send()
            .await?
            .json::<SampleResponse>()
            .await?;

        let unfinished_samples: Vec<_> = resp.items.iter().filter(|s| s.status != "Complete").collect();
        if !unfinished_samples.is_empty() {
            bail!("Project not finished yet. {} samples still processing.", unfinished_samples.len());
        }
        
        for sample in resp.items {
            let resp = client
                .get(&format!("{}/samples/{}/files", BASESPACE_URL, sample.id))
                .header("x-access-token", token)
                .send();
            file_futures.push(resp);
        }

        let mut files = vec![];
            while let Some(response) = file_futures.next().await {
            if let Ok(response) = response {
                let response = response.json::<FileResponse>().await?;
                files.extend(response.items);
            }
        }

        Ok(files)
    }

    pub fn download_files(
        &self,
        files: &[DataFile],
        project: &Project,
        output_dir: impl AsRef<Path>,
    ) -> Result<(), failure::Error> {

        if files.is_empty() {
            bail!("Selected 0 files to download");
        }

        let token = self
            .accounts
            .get(&project.user_fetched_by_id)
            .expect("Could not get token from accounts");

        let output_dir = output_dir.as_ref();
        let num_files = files.len();
        let total_size: i64 = files.iter().map(|file| file.size).sum();
        let index = AtomicUsize::new(1);
        let time_before = Instant::now();
        let pb = ProgressBar::new(total_size as u64);
        pb.set_style(indicatif::ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .progress_chars("#>-"));

        let errors: Vec<Result<(), failure::Error>> = files.par_iter()
            .map(|file| {

                let index = index.fetch_add(1, Ordering::SeqCst);
                pb.println(
                    &format!(
                        "{} {} : {}",
                        style(&format!("[{}/{}]", index, num_files)).bold().dim(),
                        &file.name,
                        convert(file.size as f64)
                    )
                );

                let client = reqwest::blocking::Client::new();
                let mut resp = client
                    .get(&format!("{}/files/{}/content", BASESPACE_URL, file.id))
                    .header("x-access-token", token)
                    .send()?;
                
                let output = output_dir.join(&file.name);
                let mut writer = BufWriter::new(File::create(&output)?);

                loop {
                    let mut buffer = vec![0; 1024];
                    let bcount = resp.read(&mut buffer[..]).unwrap();
                    pb.inc(bcount as u64);
                    buffer.truncate(bcount);
                    if !buffer.is_empty() {
                        writer.write_all(&buffer)?;
                    } else {
                        break;
                    }
                }

                if fs::metadata(&output)?.len() != file.size as u64 {
                    bail!("{} did not match expected file size.", file.name);
                }

                Ok(())
            })
            .filter(|res| res.is_err())
            .collect();

        pb.finish_and_clear();
        let elapsed = time_before.elapsed().as_millis();

        if elapsed > 0 {
            let speed = ((total_size as f64) / (elapsed as f64)) * 1000.0;

            if errors.is_empty() {
                eprintln!(
                    "{} Downloaded {} files at {}/s",
                    style("success:").bold().green(),
                    num_files,
                    convert(speed)
                );
            }
            else {
                eprintln!(
                    "{} Download {} files at {}/s, but there were {} errors.",
                    style("warning:").bold().yellow(),
                    num_files,
                    convert(speed),
                    errors.len()
                );
            }
        }

        Ok(())
    }
}

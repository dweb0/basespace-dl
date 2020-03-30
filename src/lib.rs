#[macro_use]
extern crate serde_struct_wrapper;

#[macro_use]
extern crate text_io;

pub mod api;
pub mod util;
pub mod workspace;

use api::*;
use console::style;
use crossbeam_channel::unbounded;
use failure::bail;
use futures::prelude::*;
use futures::stream::futures_unordered::FuturesUnordered;
use indicatif::ProgressBar;
use log::info;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

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

            let mut undetermined_samples: Vec<_> = resp
                .items
                .iter()
                .filter(|x| match &x.experiment_name {
                    Some(experiment_name) => {
                        experiment_name == &project.name || experiment_name.trim() == &project.name
                    }
                    None => false,
                })
                .collect();

            let undetermined_sample = if undetermined_samples.is_empty() {
                bail!(
                    "Could not find Undetermined sample for project {}",
                    project.name
                );
            } else if undetermined_samples.len() > 1 {
                util::resolve_duplicate_unindexed_reads(undetermined_samples)
            } else {
                undetermined_samples.remove(0)
            };

            let resp = client
                .get(&format!(
                    "{}/samples/{}/files",
                    BASESPACE_URL, undetermined_sample.id
                ))
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

        let unfinished_samples: Vec<_> = resp
            .items
            .iter()
            .filter(|s| s.status != "Complete")
            .collect();

        if !unfinished_samples.is_empty() {
            bail!(
                "Project not finished yet. {} samples still processing.",
                unfinished_samples.len()
            );
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
        let pb = ProgressBar::new(num_files as u64);

        // Catch badly download files, so we can inform the user
        // afterwards that they need to download them again
        let (tx, rx) = unbounded::<DataFile>();
        let catcher = std::thread::spawn(move || {
            let mut files = vec![];
            while let Ok(file) = rx.recv() {
                files.push(file);
            }
            files
        });

        let errors: Vec<failure::Error> = files
            .par_iter()
            .map(|file| {
                let index = index.fetch_add(1, Ordering::SeqCst);
                pb.println(&format!(
                    "{:<9} {:>4}  {}",
                    style(&format!("[{}/{}]", index, num_files)).bold().dim(),
                    util::convert_bytes(file.size as f64),
                    &file.name,
                ));

                let client = reqwest::blocking::Client::new();
                let mut resp = client
                    .get(&format!("{}/files/{}/content", BASESPACE_URL, file.id))
                    .header("x-access-token", token)
                    .send()?;
                let output = output_dir.join(&file.name);

                // Need separate scope since we need to close
                // the file before calculating the etag.
                {
                    let mut writer = BufWriter::new(File::create(&output)?);
                    loop {
                        let mut buffer = vec![0; 1024];
                        let bcount = resp.read(&mut buffer[..]).unwrap();
                        buffer.truncate(bcount);
                        if !buffer.is_empty() {
                            writer.write_all(&buffer).unwrap();
                        } else {
                            pb.inc(1);
                            break;
                        }
                    }
                }

                if fs::metadata(&output)?.len() != file.size as u64 {
                    tx.send(file.clone()).unwrap();
                    bail!("{} did not match expected file size.", file.name);
                }

                Ok(())
            })
            .filter_map(|res| res.err())
            .collect();

        drop(tx);
        pb.finish_and_clear();
        let elapsed = time_before.elapsed().as_millis();
        let bad_files = catcher.join().unwrap();

        if elapsed > 0 {
            let speed = ((total_size as f64) / (elapsed as f64)) * 1000.0;

            if errors.is_empty() {
                eprintln!(
                    "{} Downloaded {} files at {}/s",
                    style("success:").bold().green(),
                    num_files,
                    util::convert_bytes(speed)
                );
            } else {
                eprintln!(
                    "{} Download {} files at {}/s, but there were {} errors.",
                    style("warning:").bold().yellow(),
                    num_files,
                    util::convert_bytes(speed),
                    errors.len()
                );
                for error in errors {
                    eprintln!("{}", error);
                }
                if !bad_files.is_empty() {
                    let log_file = std::env::temp_dir().join("bdl_last_failed_download");
                    let mut writer = File::create(&log_file)
                        .expect("Could not create log file for badly formatted files");
                    for file in bad_files {
                        writeln!(&mut writer, "{}", file.name).unwrap();
                    }
                    eprintln!(
                        "{} Files stored in {}. You can retry downloading \
                         just these files using the -f argument",
                        style("tip:").bold().cyan(),
                        log_file.to_str().unwrap()
                    );
                }
            }
        }

        Ok(())
    }
}

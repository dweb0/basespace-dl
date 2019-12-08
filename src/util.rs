use super::api::{Project, Sample};
use console::style;
use failure::bail;
use number_prefix::{NumberPrefix, PrefixNames, Prefixed, Standalone};
use std::io::{Read, Write};
use tabwriter::TabWriter;

/// Return the top 5 closest matching hits for a given query
///
/// derived from clap/src/parse/features/suggestions.rs
pub fn did_you_mean<T, I>(v: &str, possible_values: I) -> Vec<String>
where
    T: AsRef<str>,
    I: IntoIterator<Item = T>,
{
    let mut candidates = vec![];
    for pv in possible_values {
        let confidence = strsim::jaro_winkler(v, pv.as_ref());
        if confidence >= 0.80 {
            candidates.push((confidence, pv.as_ref().to_owned()));
        }
    }

    candidates.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .reverse()
    });
    candidates.into_iter().take(5).map(|x| x.1).collect()
}

/// When there are duplicate projects, the user needs
/// to resolve the conflict. This function prompts the user to pick
/// the desired project
pub fn resolve_duplicate_projects(mut projects: Vec<&Project>) -> &Project {
    eprintln!(
        "{} Found {} projects with the same name.",
        style("warning:").bold().yellow(),
        projects.len()
    );

    let stderr = std::io::stderr();
    let mut stderr = stderr.lock();
    let mut writer = TabWriter::new(&mut stderr);

    writeln!(&mut writer, "#\tname\tdate created").unwrap();
    for (index, project) in projects.iter().enumerate() {
        writeln!(
            &mut writer,
            "{}\t{}\t{}",
            index, project.user_owned_by.name, project.date_created
        )
        .unwrap();
    }
    writer.flush().unwrap();

    let invalid_input = format!(
        "{} Please enter an integer from 0 to {}",
        style("error:").bold().red(),
        projects.len() - 1
    );

    let user_index = loop {
        eprint!("Enter the project index [0..{}]: ", projects.len() - 1);
        let response: Result<usize, _> = try_read!();
        break match response {
            Ok(response) => {
                if response > projects.len() - 1 {
                    eprintln!("{}", invalid_input);
                    continue;
                }
                response
            }
            Err(_) => {
                eprintln!("{}", invalid_input);
                continue;
            }
        };
    };
    projects.remove(user_index)
}

/// If a project is rerun, a separate set of Undetermined files
/// is created under the same project name. The only way to differentiate
/// is by data, so we need the user to make the decision.
pub fn resolve_duplicate_unindexed_reads(mut samples: Vec<&Sample>) -> &Sample {
    eprintln!(
        "{} Found {} \"Unindexed Reads\" with the same project name.",
        style("warning:").bold().yellow(),
        samples.len()
    );

    let stderr = std::io::stderr();
    let mut stderr = stderr.lock();
    let mut writer = TabWriter::new(&mut stderr);

    writeln!(&mut writer, "#\tname\tdate created").unwrap();
    for (index, sample) in samples.iter().enumerate() {
        writeln!(
            &mut writer,
            "{}\t{}\t{}",
            index, sample.name, sample.date_created
        )
        .unwrap();
    }
    writer.flush().unwrap();

    let invalid_input = format!(
        "{} Please enter an integer from 0 to {}",
        style("error:").bold().red(),
        samples.len() - 1
    );

    let user_index = loop {
        eprint!("Enter the sample index [0..{}]: ", samples.len() - 1);
        let response: Result<usize, _> = try_read!();
        break match response {
            Ok(response) => {
                if response > samples.len() - 1 {
                    eprintln!("{}", invalid_input);
                    continue;
                }
                response
            }
            Err(_) => {
                eprintln!("{}", invalid_input);
                continue;
            }
        };
    };

    samples.remove(user_index)
}

/// Calculate s3 etag from known part size
pub fn s3_etag(
    mut rdr: impl Read,
    file_size: usize,
    part_size: usize,
) -> Result<String, failure::Error> {

    if file_size <= part_size {
        let mut buffer = vec![0; file_size];
        rdr.read_exact(&mut buffer[..]).unwrap();
        let digest = md5::compute(&buffer);
        return Ok(format!("{:?}", digest));
    }

    let mut digests: Vec<u8> = Vec::new();
    let mut parts = 0;

    loop {
        let mut buffer = vec![0; part_size];
        let bcount = rdr.read(&mut buffer[..]).unwrap();
        if bcount == 0 {
            break;
        }
        buffer.truncate(bcount);
        let digest = md5::compute(&buffer);
        digests.extend(&digest.0);
        parts += 1;
        if buffer.is_empty() {
            break;
        }
    }

    if digests.is_empty() || parts < 2 {
        bail!("Could not calculate etag.");
    } else {
        Ok(format!("{:?}-{}", md5::compute(digests.as_slice()), parts))
    }
}
 
/// Calculate the s3 etag from path, and compare it with
/// the expected etag.
///
/// The advantage of this function is that we don't need to know
/// the etag part size (if we already know the file size and known etag).
/// 
/// TODO: part size calculation is not working. Do not use this function
pub fn verify_s3_etag(
    mut rdr: impl Read,
    expected_etag: &str,
    file_size: u64,
) -> Result<bool, failure::Error> {
    let num_parts = match expected_etag.find('-') {
        Some(index) => expected_etag
            .chars()
            .skip(index + 1)
            .collect::<String>()
            .parse::<usize>()?,
        None => {
            let mut buffer = vec![0; file_size as usize];
            rdr.read_exact(&mut buffer[..]).unwrap();
            let digest = md5::compute(&buffer);
            let actual_etag = format!("{:?}", digest);
            return Ok(actual_etag == expected_etag);
        }
    };

    // Assumes AWS part sizes are a factor of one megabyte
    static ONE_MEGABYTE: f64 = 1024.0 * 1024.0;

    // TODO: does not work 100% of the time. 
    let x = file_size as f64 / num_parts as f64;
    let y = x % (ONE_MEGABYTE);
    let part_size = (x - y + (ONE_MEGABYTE)) as usize;

    let mut digests: Vec<u8> = Vec::new();
    let mut parts = 0;

    loop {
        let mut buffer = vec![0; part_size];
        let bcount = rdr.read(&mut buffer[..]).unwrap();
        if bcount == 0 {
            break;
        }
        buffer.truncate(bcount);
        let digest = md5::compute(&buffer);
        digests.extend(&digest.0);
        parts += 1;
        if buffer.is_empty() {
            break;
        }
    }

    let actual_etag = if digests.is_empty() || parts < 2 {
        bail!("Could not calculate etag.");
    } else {
        format!("{:?}-{}", md5::compute(digests.as_slice()), parts)
    };

    Ok(actual_etag == expected_etag)
}

/// Convert bytes to human readable form.
///
/// Trying to match format of unix's "ls -lh" command
pub fn convert_bytes(num: f64) -> String {
    match NumberPrefix::decimal(num) {
        Standalone(bytes) => bytes.to_string(),
        Prefixed(prefix, n) => {
            let symbol = prefix.symbol();
            let number = if n >= 10.0 {
                format!("{:.0}", n)
            } else {
                format!("{:.1}", n)
            };
            format!("{}{}", number, symbol)
        }
    }
}

use super::api::{Project, Sample};
use console::style;

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

    for (index, project) in projects.iter().enumerate() {
        eprintln!(
            "[{}] name = \"{}\", dateCreated = \"{}\"",
            index, project.user_owned_by.name, project.date_created
        );
    }

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
        "{} Found {} \"Unindexed Reads\" with the same project name",
        style("warning:").bold().yellow(),
        samples.len()
    );

    for (index, sample) in samples.iter().enumerate() {
        eprintln!(
            "[{}] name = \"{}\", dateCreated = \"{}\"",
            index, sample.name, sample.date_created
        );
    }

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

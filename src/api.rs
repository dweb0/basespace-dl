/// Minimal API based - Only using what we need
/// https://developer.basespace.illumina.com/docs/content/documentation/rest-api/api-reference
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
#[serde(remote = "Self")]
pub struct ProjectResponse {
    items: Vec<_Project>,
}

impl ProjectResponse {
    /// Get all projects from the response and tag on
    /// the user_fetched_by field, which is required
    /// for later steps.
    pub fn projects_as_user(self, user_id: &str) -> Vec<Project> {
        self.items
            .into_iter()
            .map(|project| Project {
                name: project.name,
                id: project.id,
                user_owned_by: project.user_owned_by,
                user_fetched_by_id: user_id.to_owned(),
                date_created: project.date_created,
            })
            .collect()
    }
}

/// This is the actual schema for the API response,
/// but since we need the user_fetched_by field,
/// we will only use this temporarily
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct _Project {
    pub name: String,
    pub id: String,
    pub user_owned_by: User,
    pub date_created: String,
}

/// Contains the user_fetched_by_id, which is
/// used to access shared projects
///
/// In the previous version, we could only
/// access projects by the user_owned by
#[derive(Debug)]
pub struct Project {
    pub name: String,
    pub id: String,
    pub user_owned_by: User,
    pub user_fetched_by_id: String,
    pub date_created: String,
}

impl AsRef<str> for Project {
    fn as_ref(&self) -> &str {
        self.name.as_ref()
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
#[serde(remote = "Self")]
pub struct CurrentUserResponse {
    pub id: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct User {
    pub name: String,
    pub id: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
#[serde(remote = "Self")]
pub struct FileResponse {
    pub items: Vec<DataFile>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct DataFile {
    pub id: String,
    pub name: String,
    pub size: i64,
    pub e_tag: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
#[serde(remote = "Self")]
pub struct SampleResponse {
    pub items: Vec<Sample>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Sample {
    pub id: String,
    pub status: String,
    pub name: String,
    pub experiment_name: Option<String>,
    pub date_created: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
#[serde(remote = "Self")]
pub struct RunResponse {
    items: Vec<Run>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Run {
    pub name: String,
    pub id: String,
    pub user_owned_by: User,
    pub date_created: String,
}

deserialize_with_root!("Response": SampleResponse);
deserialize_with_root!("Response": FileResponse);
deserialize_with_root!("Response": CurrentUserResponse);
deserialize_with_root!("Response": ProjectResponse);
deserialize_with_root!("Response": RunResponse);

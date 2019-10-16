/// Minimal API based - Only using what we need
/// https://developer.basespace.illumina.com/docs/content/documentation/rest-api/api-reference
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ProjectResponse {
    response: ProjectResponseB,
}

impl ProjectResponse {
    /// Get all projects from the response and tag on
    /// the user_fetched_by field, which is required
    /// for later steps.
    pub fn projects_as_user(self, user_id: &str) -> Vec<Project> {
        self.response.items
            .into_iter()
            .map(|project| {
                Project {
                    name: project.name,
                    id: project.id,
                    user_owned_by: project.user_owned_by,
                    user_fetched_by_id: user_id.to_owned(),
                    date_created: project.date_created
                }
            })
            .collect()
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct ProjectResponseB {
    pub items: Vec<_Project>,
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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct CurrentUserResponse {
    response: CurrentUserResponseB,
}

impl CurrentUserResponse {
    pub fn user_id(&self) -> &str {
        &self.response.id
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct CurrentUserResponseB {
    id: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct User {
    pub name: String,
    pub id: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct FileResponse {
    response: FileResponseB,
}

impl FileResponse {
    pub fn files(self) -> Vec<DataFile> {
        self.response.items
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct FileResponseB {
    pub items: Vec<DataFile>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct DataFile {
    pub id: String,
    pub name: String,
    pub size: i64,
    pub e_tag: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct SampleResponse {
    response: SampleResponseB,
}

impl SampleResponse {
    pub fn samples(self) -> Vec<Sample> {
        self.response.items
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct SampleResponseB {
    pub items: Vec<Sample>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Sample {
    pub id: String,
    pub status: String,
    pub name: String,
    pub experiment_name: Option<String>,
    pub date_created: String
}

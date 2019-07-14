#[derive(Deserialize, Debug)]
pub struct ProjectResponse {
    #[serde(rename = "Response")]
    response: ProjectResponseB,
}

impl ProjectResponse {
    pub fn projects(self) -> Vec<Project> {
        self.response.items
    }
}

#[derive(Deserialize, Debug)]
struct ProjectResponseB {
    #[serde(rename = "Items")]
    pub items: Vec<Project>,
}

#[derive(Deserialize, Debug)]
pub struct Project {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "UserOwnedBy")]
    pub user_owned_by: User,
    #[serde(rename = "DateCreated")]
    pub date_created: String,
}

#[derive(Deserialize, Debug)]
pub struct CurrentUserResponse {
    #[serde(rename = "Response")]
    response: CurrentUserResponseB,
}

impl CurrentUserResponse {
    pub fn user_id(&self) -> &str {
        &self.response.id
    }
}

#[derive(Deserialize, Debug)]
struct CurrentUserResponseB {
    #[serde(rename = "Id")]
    id: String,
}

#[derive(Deserialize, Debug)]
pub struct User {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Id")]
    pub id: String,
}

#[derive(Deserialize, Debug)]
pub struct FileResponse {
    #[serde(rename = "Response")]
    response: FileResponseB,
}

impl FileResponse {
    pub fn files(self) -> Vec<DataFile> {
        self.response.items
    }
}

#[derive(Deserialize, Debug)]
struct FileResponseB {
    #[serde(rename = "Items")]
    pub items: Vec<DataFile>,
}

#[derive(Deserialize, Debug)]
pub struct DataFile {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Size")]
    pub size: i64,
}

#[derive(Deserialize, Debug)]
pub struct SampleResponse {
    #[serde(rename = "Response")]
    response: SampleResponseB,
}

impl SampleResponse {
    pub fn samples(self) -> Vec<Sample> {
        self.response.items
    }
}

#[derive(Deserialize, Debug)]
struct SampleResponseB {
    #[serde(rename = "Items")]
    pub items: Vec<Sample>,
}

#[derive(Deserialize, Debug)]
pub struct Sample {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Status")]
    pub status: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "ExperimentName")]
    pub experiment_name: Option<String>,
}

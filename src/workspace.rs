use std::fs::{self, File};
use std::io::Read;
use std::path::PathBuf;
use std::collections::HashMap;
use crate::MultiApi;

pub struct Workspace {
    pub config_file: PathBuf,
}

impl Workspace {
    pub fn new() -> Result<Workspace, failure::Error> {

        let home = dirs::home_dir().expect("Could not locate $HOME.");
        let config_dir = home.join(".config/basespace-dl");
        if !config_dir.is_dir() {
            fs::create_dir_all(&config_dir)?;
        }

        let config_file = config_dir.join("default.toml");
        if !config_file.exists() {
            File::create(&config_file)?;
        }

        Ok(Workspace { config_file })
    }

    pub fn accounts(&self) -> Result<HashMap<String, String>, failure::Error> {
        let mut file = File::open(&self.config_file)?;
        let mut buffer = String::new();
        file.read_to_string(&mut buffer)?;
        let accounts: HashMap<String, String> = toml::from_str(&buffer)?;
        Ok(accounts)
    }

    pub fn to_multiapi(self) -> Result<MultiApi, failure::Error> {
        let accounts = self.accounts()?;

        ensure!(
            accounts.keys().count() > 0,
            format_err!("{} is empty. Please add token(s).", self.config_file.to_str().unwrap())
        );

        Ok(MultiApi::new(self.accounts()?))
    }
}

use std::fs::{self, File};
use std::io::{Read, Write};
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

    fn accounts(&self) -> Result<HashMap<String, String>, failure::Error> {
        let mut file = File::open(&self.config_file)?;
        let mut buffer = String::new();
        file.read_to_string(&mut buffer)?;
        let accounts: HashMap<String, String> = toml::from_str(&buffer)?;
        Ok(accounts)
    }

    pub fn add_account(&self, user_id: &str, token: &str) -> Result<(), failure::Error> {
        let mut accounts = self.accounts()?;
        let inserted = accounts.insert(user_id.to_owned(), token.to_owned());
        if inserted.is_some() {
            bail!("User {} is already in config file.", user_id);
        }
        let buffer = toml::to_string(&accounts)?;
        let mut file = File::create(&self.config_file)?;
        write!(&mut file, "{}", buffer)?;
        Ok(())
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

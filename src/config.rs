use std::env;

use anyhow::{Context, Result};

pub struct Config {
    pub file_name: String,
    pub auth_token: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let file_name = env::var("FILE_NAME").with_context(|| "FILE_NAME must be set")?;
        let auth_token = env::var("AUTH_TOKEN").with_context(|| "AUTH_TOKEN must be set")?;
        let port: u16 = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .with_context(|| "PORT must be a number between 0 and 65535")?;

        Ok(Self {
            file_name,
            auth_token,
            port,
        })
    }
}

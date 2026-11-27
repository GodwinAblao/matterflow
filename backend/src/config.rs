use std::{env, error::Error};

pub struct Config {
    pub host: String,
    pub port: u16,
    pub frontend_origin: String,
    pub database_url: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into()),
            port: env::var("PORT").unwrap_or_else(|_| "3000".into()).parse()?,
            frontend_origin: env::var("FRONTEND_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:5173".into()),
            database_url: env::var("DATABASE_URL")
                .ok()
                .filter(|url| !url.trim().is_empty()),
        })
    }
}

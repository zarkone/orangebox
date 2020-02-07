use std::env;

pub struct Config {
    pub auth_token: String,
}

impl Config {
    pub fn new() -> Result<Config, &'static str> {
        match env::var("GITHUB_TOKEN") {
            Ok(auth_token) => Ok(Config { auth_token }),
            Err(_e) => Err("env GITHUB_TOKEN is required."),
        }
    }
}

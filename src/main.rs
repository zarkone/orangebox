extern crate orangebox;

use hyper::header::AUTHORIZATION;
use std::collections::HashMap;
use std::error::Error;

// TODO: take from .git
static REPO: &str = "zarkone/literally.el";

#[derive(std::fmt::Debug)]
struct AccessForbiddenError;

// TODO: see https://stevedonovan.github.io/rust-gentle-intro/6-error-handling.html
// impl std::fmt::Display for AccessForbiddenError {}

impl Error for AccessForbiddenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

fn make_api_url(repo: &String) -> String {
    format!(
        "https://api.github.com/repos/{repo}/actions/runs",
        repo = repo
    )
}

async fn req(
    url: String,
    conf: &orangebox::Config,
) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let username = "zarkone";
    let client = reqwest::Client::new();
    // TODO: make Client
    let encoded_token = base64::encode(&format!("{}:{}", username, conf.auth_token));
    let resp = client
        .get(&url)
        // TODO: add mandatory useragent=zarkone
        .header(AUTHORIZATION, &encoded_token)
        .send()
        .await?;
    println!("{:?}, {:?}", encoded_token, resp);

    if 403 == resp.status() {
        eprintln!("{}", resp.text().await?);
        return Err(Box::new(AccessForbiddenError));
    }

    return Ok(resp.json::<HashMap<String, String>>().await?);
}

#[tokio::main]
async fn main() -> Result<(), &'static str> {
    let conf = match orangebox::Config::new() {
        Ok(conf) => conf,
        Err(e) => return Err(e),
    };

    println!("auth_token:: {}", conf.auth_token);

    let url = make_api_url(&REPO.to_string());
    println!("{:?}", url);
    match req(url.to_string(), &conf).await {
        Ok(resp) => println!("{:#?}", resp),
        Err(e) => println!("{}", e),
    }
    Ok(())
}

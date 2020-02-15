extern crate orangebox;

use hyper::header::{AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::string::String;

// TODO: take from .git
static REPO: &str = "zarkone/literally.el";

#[derive(std::fmt::Debug)]

enum WorkflowConclusion {
    Success,
    Failure,
    Cancelled,
}

#[derive(std::fmt::Debug)]
struct UnknownConclusionError {
    details: String,
}

impl fmt::Display for UnknownConclusionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}
impl Error for UnknownConclusionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl serde::de::Error for UnknownConclusionError {
    fn custom<S: ToString>(msg: S) -> Self {
        UnknownConclusionError {
            details: msg.to_string(),
        }
    }
}

impl<'de> Deserialize<'de> for WorkflowConclusion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "success" => Ok(WorkflowConclusion::Success),
            "failure" => Ok(WorkflowConclusion::Failure),
            "cancelled" => Ok(WorkflowConclusion::Cancelled),
            conclusion => Err(serde::de::Error::custom(conclusion)),
        }
    }
}

struct WorkflowRun {
    logs_url: String,
    conclusion: WorkflowConclusion,
}

#[derive(std::fmt::Debug)]
struct AccessForbiddenError {
    details: String,
}

impl AccessForbiddenError {
    fn new(msg: &str) -> AccessForbiddenError {
        AccessForbiddenError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for AccessForbiddenError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}
impl Error for AccessForbiddenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
// jobs_url
// "status": "completed",
// "conclusion": "success",

//
type GithubResponse = HashMap<String, String>;

fn make_api_url(repo: &String) -> String {
    format!(
        "https://api.github.com/repos/{repo}/actions/runs",
        repo = repo
    )
}

async fn req(url: String, conf: &orangebox::Config) -> Result<GithubResponse, Box<dyn Error>> {
    let username = "zarkone";
    let client = reqwest::Client::new();
    let encoded_token = base64::encode(&format!("{}:{}", username, conf.auth_token));
    let resp = client
        .get(&url)
        .header(AUTHORIZATION, &encoded_token)
        .header(USER_AGENT, username)
        .send()
        .await?;

    println!("TOKEN::: {:?}, {:?}", &encoded_token, resp);

    if 403 == resp.status() {
        let e = AccessForbiddenError::new(&resp.text().await?);
        return Err(Box::new(e));
    }

    return Ok(resp.json::<GithubResponse>().await?);
}

#[tokio::main]
async fn main() -> Result<(), &'static str> {
    let conf = match orangebox::Config::new() {
        Ok(conf) => conf,
        Err(e) => return Err(e),
    };

    // println!("auth_token:: {}", conf.auth_token);

    let url = make_api_url(&REPO.to_string());
    // println!("{:?}", url);
    match req(url.to_string(), &conf).await {
        Ok(resp) => println!("Success: \n {:#?}", resp),
        Err(e) => eprintln!("{:?}", e),
    }
    Ok(())
}

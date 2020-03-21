extern crate orangebox;

use bytes::Bytes;
use hyper::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Deserializer};
use std::error::Error;
use std::fmt;
use std::io::Cursor;
use std::string::String;
use zip::read::ZipArchive;

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

#[derive(Debug, Deserialize)]
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
#[derive(Debug, Deserialize)]
struct WorkflowRuns {
    total_count: u32,
    workflow_runs: Vec<WorkflowRun>,
}

fn make_api_url(repo: &String) -> String {
    format!(
        "https://api.github.com/repos/{repo}/actions/runs",
        repo = repo
    )
}

// Performs http request tospecified GitHub API `url`. Uses `conf` for API authorization
async fn req<T>(url: String, conf: &orangebox::Config) -> Result<T, Box<dyn Error>>
where
    T: for<'de> Deserialize<'de>,
{
    let username = "zarkone";
    let client = reqwest::Client::new();
    let encoded_token = base64::encode(&format!("{}:{}", username, conf.auth_token));
    let resp = client
        .get(&url)
        .header(AUTHORIZATION, &encoded_token)
        .header(ACCEPT, "application/vnd.github.antiope-preview+json")
        .header(USER_AGENT, username)
        .send()
        .await?;

    println!("TOKEN::: {:?}, {:?}", &encoded_token, resp);

    if 403 == resp.status() {
        let e = AccessForbiddenError::new(&resp.text().await?);
        return Err(Box::new(e));
    }

    return Ok(resp.json::<T>().await?);
}

async fn req_zip(
    url: &String,
    conf: &orangebox::Config,
) -> Result<ZipArchive<Cursor<Bytes>>, Box<dyn Error>> {
    let username = "zarkone";
    let client = reqwest::Client::new();
    let encoded_token = base64::encode(&format!("{}:{}", username, conf.auth_token));
    let resp = client
        .get(url)
        .header(AUTHORIZATION, &encoded_token)
        .header(ACCEPT, "application/vnd.github.antiope-preview+json")
        .header(USER_AGENT, username)
        .send()
        .await?;

    println!("-------- ZIP ---------- ");
    println!("TOKEN::: {:?}, {:?}", &encoded_token, resp);
    println!("URL::: {:?}", url);

    if 403 == resp.status() {
        let e = AccessForbiddenError::new(&resp.text().await?);
        return Err(Box::new(e));
    }

    let zip_bytes = resp.bytes().await?;
    let reader = Cursor::new(zip_bytes);

    Ok(ZipArchive::new(reader)?)
}

// struct Logs {
//     text: String,
// }

// async fn get_last_run_logs(response: &WorkflowRuns, conf: &orangebox::Config) -> Option<Logs> {
//     if response.total_count > 0 {
//         let last_run = response.workflow_runs[0].logs_url;
//         let zip_file = req(last_run, conf);
//         return Some();
//     } else {
//         return None;
//     }
// }

#[tokio::main]
async fn main() -> Result<(), &'static str> {
    let conf = orangebox::Config::new()?;

    let url = make_api_url(&REPO.to_string());

    let workflow_runs: WorkflowRuns = match req::<WorkflowRuns>(url.to_string(), &conf).await {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Error: \n {}", (*e).to_string());
            return Err("Error");
        }
    };

    println!("{:?}", workflow_runs);

    let last_run_logs_url = &workflow_runs.workflow_runs[0].logs_url;
    let logs_zip: ZipArchive<Cursor<Bytes>> = match req_zip(last_run_logs_url, &conf).await {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Error: \n {}", (*e).to_string());
            return Err("Error");
        }
    };

    for fname in logs_zip.file_names() {
        println!("Zip file: {:?}", fname);
    }

    Ok(())
}

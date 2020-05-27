extern crate orangebox;

use bytes::Bytes;
use hyper::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, LOCATION, USER_AGENT};
use serde::{Deserialize, Deserializer};
use std::error::Error;
use std::fmt;
use std::io::{Cursor, Read};
use std::string::String;
use zip::read::{ZipArchive, ZipFile};

// TODO: take from .git
static REPO: &str = "zarkone/literally.el";

#[derive(std::fmt::Debug, PartialEq)]
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
    jobs_url: String,
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

#[derive(std::fmt::Debug)]
struct UnexpectedReplyError {
    details: String,
}

impl UnexpectedReplyError {
    fn new(msg: &str) -> UnexpectedReplyError {
        UnexpectedReplyError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for UnexpectedReplyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for UnexpectedReplyError {
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

#[derive(Debug, Deserialize)]
struct Step {
    name: String,
    status: String,
    conclusion: String,
    number: u32,
}

#[derive(Debug, Deserialize)]
struct Job {
    name: String,
    status: String,
    conclusion: String,
    steps: Vec<Step>,
}

#[derive(Debug, Deserialize)]
struct Jobs {
    total_count: u32,
    jobs: Vec<Job>,
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
    let encoded_token = format!(
        "Basic {}",
        base64::encode(&format!("{}:{}", username, conf.auth_token))
    );

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
    let encoded_token = format!(
        "Basic {}",
        base64::encode(&format!("{}:{}", username, conf.auth_token))
    );

    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&encoded_token)?);
    headers.insert(USER_AGENT, HeaderValue::from_static("zarkone"));
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/vnd.github.antiope-preview+json"),
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    println!("{:?}", client.get(url));

    let resp: reqwest::Response = client.get(url).send().await?;

    println!("TOKEN::: {:?}, {:?}", &encoded_token, resp);

    if 403 == resp.status() {
        let e = AccessForbiddenError::new(&resp.text().await?);
        return Err(Box::new(e));
    }

    if 302 == resp.status() {
        let zip_location = String::from(resp.headers()[LOCATION].to_str()?);
        let resp: reqwest::Response = client.get(&zip_location).send().await?;
        let zip_bytes = resp.bytes().await?;
        let reader = Cursor::new(zip_bytes);

        return Ok(ZipArchive::new(reader)?);
    } else {
        let e = UnexpectedReplyError::new(&resp.text().await?);
        return Err(Box::new(e));
    }
}

fn print_file(file: &mut ZipFile) -> Result<(), &'static str> {
    for byte in file.bytes() {
        match byte {
            Ok(b) => print!("{}", char::from(b)),
            Err(e) => {
                eprintln!("Error: \n {}", e.to_string());
                return Err("Error");
            }
        };
    }
    Ok(())
}

fn take_first<'p, T>(items: &'p Vec<T>, pred: &dyn Fn(&'p T) -> bool) -> Option<&'p T> {
    for item in items.iter() {
        if pred(item) {
            return Some(item);
        }
    }

    return None;
}

#[tokio::main]
async fn main() -> Result<(), &'static str> {
    env_logger::init();
    let conf = orangebox::Config::new()?;

    let url = make_api_url(&REPO.to_string());

    let workflow_runs: WorkflowRuns = match req::<WorkflowRuns>(url.to_string(), &conf).await {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Error: \n {}", (*e).to_string());
            return Err("Error");
        }
    };

    fn is_failed(run: &WorkflowRun) -> bool {
        run.conclusion == WorkflowConclusion::Failure
    }

    let last_failed_run = take_first(&workflow_runs.workflow_runs, &is_failed).unwrap();

    println!("{:?}", last_failed_run.jobs_url);

    let jobs = match req::<Jobs>(last_failed_run.jobs_url.to_string(), &conf).await {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Error: \n {}", (*e).to_string());
            return Err("Error");
        }
    };

    // TODO: implement trait for "conclusion"
    let failed_job = take_first(&jobs.jobs, &is_failed);
    println!("JOBS: {:?}", &jobs);

    let mut logs_zip: ZipArchive<Cursor<Bytes>> =
        match req_zip(&last_failed_run.logs_url, &conf).await {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("Error: \n {}", (*e).to_string());
                return Err("Error");
            }
        };

    // for i in 0..logs_zip.len() {}
    // let a = &workflow_runs.workflow_runs[0];

    let mut file = logs_zip.by_index(0).unwrap();
    println!("Filename: {}", file.name());

    print_file(&mut file)?;

    Ok(())
}

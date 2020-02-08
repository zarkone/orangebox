extern crate orangebox;
use std::collections::HashMap;

async fn req(url: String) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let resp = reqwest::get(&url)
        .await?
        .json::<HashMap<String, String>>()
        .await?;
    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), &'static str> {
    let conf = match orangebox::Config::new() {
        Ok(conf) => conf,
        Err(e) => return Err(e),
    };

    println!("auth_token:: {}", conf.auth_token);

    match req("https://httpbin.org/ip".to_string()).await {
        Ok(resp) => println!("{:#?}", resp),
        Err(_) => println!("error."),
    }
    Ok(())
}

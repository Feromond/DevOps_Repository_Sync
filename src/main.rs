use reqwest::Client;
use serde::Deserialize;
use serde_json;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

// Struct to hold the configuration
#[derive(Deserialize)]
struct Config {
    repo_path: String,
    remote_url: String,
    pat: String,
}
// Grabs API response and deserializes it into the struct
#[derive(Deserialize)]
struct ApiResponse {
    value: Vec<Commit>,
}
// Deserializes the commitId in the api response array into a string and renames to snake case
#[derive(Deserialize)]
struct Commit {
    #[serde(rename = "commitId")]
    commit_id: String,
}

// Reads the config file and parses it into the Config struct
fn read_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_path = Path::new("config.toml");

    if !config_path.exists() {
        eprintln!("Config file not found in the same directory as the executable. Please ensure 'config.toml' is present.");

        // Prompt the user to press Enter before exiting
        print!("Press Enter to exit...");
        io::stdout().flush()?; // Ensure the message is printed before reading input
        let _ = io::stdin().read_line(&mut String::new());

        std::process::exit(1); // Exit the program with a non-zero status
    }

    let config_content = fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&config_content)?;
    Ok(config)
}

// Checks the latest commit hash / id on the remote azure
async fn get_latest_commit(
    remote_url: &str,
    pat: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client
        .get(remote_url)
        .basic_auth("", Some(pat))
        .send()
        .await?
        .text()
        .await?;

    // Converts the string response from the API into a JSON
    let api_response: ApiResponse = serde_json::from_str(&response)?;

    // Grabbing first commit in the array to check most recent commit on Main
    Ok(api_response.value[0].commit_id.clone())
}

// Checks the local commit head hash / id to then compare with the remote version
fn get_local_commit(repo_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("rev-parse")
        .arg("HEAD")
        .output()?;
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn pull_changes(repo_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("pull")
        .status()?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = read_config()?;

    loop {
        let remote_commit = get_latest_commit(&config.remote_url, &config.pat).await?;
        let local_commit = get_local_commit(&config.repo_path)?;

        if remote_commit != local_commit {
            println!("New changes detected. Pulling updates...");
            pull_changes(&config.repo_path)?;
        } else {
            println!("No new changes.");
        }

        sleep(Duration::from_secs(300)).await; // Check every 5 minutes
    }
}

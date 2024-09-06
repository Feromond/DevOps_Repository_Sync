use chrono::{DateTime, Utc};
use log::{error, info};
use reqwest::Client;
use serde::Deserialize;
use serde_json;
use simplelog::*;
use std::fs;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime};
use tokio::time::sleep;

// Struct to hold the configuration
#[derive(Deserialize)]
struct AppConfig {
    repo_path: String,
    remote_url: String,
    pat: String,
    check_interval_seconds: u64,
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

// Reads the config file and parses it into the AppConfig struct
fn read_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    let config_path = Path::new("config.toml");

    if !config_path.exists() {
        error!("Config file not found.");
        eprintln!("Config file not found in the same directory as the executable. Please ensure 'config.toml' is present.");

        // Prompt the user to press Enter before exiting
        print!("Press Enter to exit...");
        io::stdout().flush()?; // Ensure the message is printed before reading input
        let _ = io::stdin().read_line(&mut String::new());

        std::process::exit(1); // Exit the program with a non-zero status
    }

    let config_content = fs::read_to_string(config_path)?;
    let config: AppConfig = toml::from_str(&config_content)?;
    info!("Config file read successfully.");
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
        .await?;

    info!("API request sent successfully.");

    let response_text = response.text().await?;
    let api_response: ApiResponse = serde_json::from_str(&response_text)?;
    info!("Received latest commit from remote.");

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

    let commit_id = String::from_utf8(output.stdout)?.trim().to_string();
    info!("Local commit ID: {}", commit_id);

    Ok(commit_id)
}

fn pull_changes(repo_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("pull")
        .status()?;

    if status.success() {
        info!("Changes pulled successfully.");
    } else {
        error!("Failed to pull changes.");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging to a file
    CombinedLogger::init(vec![WriteLogger::new(
        LevelFilter::Info,
        simplelog::Config::default(),
        File::create("app.log").unwrap(),
    )])?;

    info!("Starting application");

    let config = read_config()?;
    let mut last_change_time = SystemTime::now();

    loop {
        match get_latest_commit(&config.remote_url, &config.pat).await {
            Ok(remote_commit) => match get_local_commit(&config.repo_path) {
                Ok(local_commit) => {
                    if remote_commit != local_commit {
                        info!("New changes detected. Pulling updates...");
                        if let Err(e) = pull_changes(&config.repo_path) {
                            error!("Failed to pull changes: {}", e);
                        } else {
                            last_change_time = SystemTime::now();
                        }
                    } else {
                        let elapsed = last_change_time.elapsed()?.as_secs();
                        let last_change_time: DateTime<Utc> = last_change_time.into();
                        let formatted_time = last_change_time.format("%Y-%m-%d %H:%M:%S");
                        print!(
                            "\rNo new changes since {}. Elapsed time: {} seconds.",
                            formatted_time, elapsed
                        );
                        io::stdout().flush()?;
                    }
                }
                Err(e) => {
                    error!("Failed to get local commit: {}", e);
                }
            },
            Err(e) => {
                error!("Failed to get latest commit from remote: {}", e);
            }
        }

        sleep(Duration::from_secs(config.check_interval_seconds)).await;
    }
}

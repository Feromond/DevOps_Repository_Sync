use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use serde_json;
use std::fs::{self, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime};
use tokio::time::sleep;

// Struct to hold the configuration
#[derive(Deserialize)]
struct Config {
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

// Deserializes the commitId in the API response array into a string and renames to snake case
#[derive(Deserialize)]
struct Commit {
    #[serde(rename = "commitId")]
    commit_id: String,
}

// Function to log messages
fn log_message(message: &str, log_file: &mut BufWriter<std::fs::File>) {
    let log_entry = format!(
        "{}: {}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        message
    );
    log_file
        .write_all(log_entry.as_bytes())
        .expect("Failed to write to log file");
    log_file.flush().expect("Failed to flush log file");
}

// Reads the config file and parses it into the Config struct
fn read_config(
    log_file: &mut BufWriter<std::fs::File>,
) -> Result<Config, Box<dyn std::error::Error>> {
    let config_path = Path::new("config.toml");

    if !config_path.exists() {
        log_message("Config file not found.", log_file);
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

// Checks the latest commit hash / id on the remote Azure
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

// Pulls changes from the remote repository
fn pull_changes(
    repo_path: &str,
    log_file: &mut BufWriter<std::fs::File>,
) -> Result<(), Box<dyn std::error::Error>> {
    log_message(
        &format!("Marking directory as safe: {}", repo_path),
        log_file,
    );

    // Use the --system flag to set the safe directory at the system level
    let safe_dir_command = Command::new("git")
        .arg("config")
        .arg("--system")
        .arg("--add")
        .arg("safe.directory")
        .arg(repo_path)
        .status();

    match safe_dir_command {
        Ok(status) if status.success() => {}
        Ok(status) => {
            log_message(
                &format!(
                    "Failed to mark directory as safe at the system level. Exit code: {}",
                    status
                ),
                log_file,
            );
            return Err(Box::new(io::Error::new(
                io::ErrorKind::Other,
                "Failed to mark directory as safe at the system level",
            )));
        }
        Err(e) => {
            log_message(
                &format!("Failed to run 'git config' system command: {}", e),
                log_file,
            );
            return Err(Box::new(e));
        }
    }

    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("pull")
        .output()?;

    if !output.status.success() {
        log_message(
            &format!(
                "git pull failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
            log_file,
        );
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            "git pull failed",
        )));
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open or create the log file
    let log_file_path = "script_log.txt";
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path)
        .expect("Failed to open log file");
    let mut log_file = BufWriter::new(log_file);

    log_message("Script started.", &mut log_file);

    let config = read_config(&mut log_file)?;
    let mut last_change_time = SystemTime::now();
    let mut first_check_done = false;

    loop {
        let remote_commit = get_latest_commit(&config.remote_url, &config.pat).await?;
        let local_commit = get_local_commit(&config.repo_path)?;

        if !first_check_done {
            // Log the first time we check for changes
            log_message("First commit check done.", &mut log_file);
            first_check_done = true;
        }

        if remote_commit != local_commit {
            log_message("New changes detected. Pulling updates...", &mut log_file);
            print!("New changes detected. Pulling updates...\n");
            pull_changes(&config.repo_path, &mut log_file)?;
            last_change_time = SystemTime::now();
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

        sleep(Duration::from_secs(config.check_interval_seconds)).await;
    }
}

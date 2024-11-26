use std::{collections::HashMap, fs::File, path::PathBuf, time::Duration};

use clap::{command, Parser};
use color_eyre::Section;
use eyre::{Context, Result};
use git2::Repository;
use indicatif::ProgressBar;
use thiserror::Error;
use serde::{Deserialize, Serialize};
use tracing::{info, level_filters::LevelFilter, warn};



/// Simple tool to clean emails from the patches of a given lsit of git repositories.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {

    /// The configuration file to load from.
    /// This should be a JSON file in the following format:
    /// {
    ///     "repositories": ["Org/reponame"...],
    ///     "substitutions": {
    ///         "email@example.com": {
    ///             "new_email": "another_email@example.com",
    ///             "new_author": "Author Name"
    ///         }
    ///     }
    /// }
    config: PathBuf,
    /// Set this flag after running the command the first time. It will force push every repository that was previously modified.
    #[arg(long)]
    commit: bool,
    /// Set this flag to sign all commits with your default GPG signing key.
    /// WARNING: This will sign *every* commit, including those not made by you!
    #[arg(long)]
    sign: bool,
}


#[derive(Deserialize, Debug)]
struct Substitution {
    new_email: String,
    new_author: String,
}

#[derive(Deserialize, Debug)]
struct Config {
    repositories: Vec<String>,
    substitutions: HashMap<String, Substitution>

}

#[derive(Debug, Error)]
#[error("{0}")]
struct StringError(String);

fn main() -> Result<()> {
    color_eyre::install()?;

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .parse("").unwrap()
        )
        .try_init().unwrap();

    // Parse args
    let args = Args::parse();
    

    // Load config file
    let conf: Config = serde_json::from_reader(
        File::open(args.config.clone())
            .wrap_err(format!("Unable to open configuration file {:?}", args.config))?
    ).wrap_err("Error reading configuration file")?;

    // Construct the base path
    let base = std::env::current_dir().unwrap()
        .join("cleaner");
    
    // The path repos will be put in
    let repos = base.join("repos");

    // The path backups will be put in
    let backups = base.join("backups");


    // If not commiting, pull each repo and backup
    if !args.commit {
        info!("Processing repositories");
        // Process each repository
        for repo in conf.repositories {

            let spin = ProgressBar::new_spinner()
                .with_message(format!("Processing {repo}"));
            spin.enable_steady_tick(Duration::from_millis(100));

            // Construct the repo URL
            let url = format!("git@github.com:{}.git", repo);

            // Clone the repository
            let repository = match Repository::clone(&url, repos.join(repo.clone())) {
                Ok(r) => r,
                Err(e) => {
                    warn!("Received error while cloning {repo}:\n{e}");
                    warn!("Skipping cloning {url}");
                    continue;
                },
            };

            println!("{:?}", repository.remotes().map(|v| v.iter().filter_map(|v| v.map(|v| v.to_owned())).collect::<Vec<_>>()));
            
            spin.finish_with_message(format!("Finished processing {repo}"));
        }
    } else {
        println!("{args:?}\n{conf:?}");
    }
    


    Ok(())
}

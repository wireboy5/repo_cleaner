use std::{collections::HashMap, fs::File, path::PathBuf};

use clap::{command, Parser};
use serde::{Deserialize, Serialize};



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

fn main() {

    // Parse args
    let args = Args::parse();
    

    // Load config file
    let conf: Config = serde_json::from_reader(File::open(args.config).unwrap());

    println!("Hello, world!");
}

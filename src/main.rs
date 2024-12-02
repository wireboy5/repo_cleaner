use std::{collections::HashMap, fs::File, path::{Path, PathBuf}, process::Command, time::Duration};

use clap::{command, Parser};
use eyre::{Context, Result};
use git2::{ErrorCode, Repository};
use git2_credentials::CredentialHandler;
use indicatif::ProgressBar;
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


#[derive(Deserialize, Serialize, Debug)]
struct Substitution {
    new_email: String,
    new_author: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct Config {
    repositories: Vec<String>,
    email_substitutions: HashMap<String, String>,
    name_substitutions: HashMap<String, String>,

}

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

    // Dump the substitutions to a json map mapping old email to new email
    let emails = serde_json::to_string(&conf.email_substitutions).unwrap();
    let names = serde_json::to_string(&conf.name_substitutions).unwrap();
    
    // Create python cleaner for git-filter-repo
    let email_cleaner = format!("return email if email.decode() not in \"{}\" else {}[email.decode()].encode()",
        conf.email_substitutions.keys()
            .map(|v| v.clone()).collect::<Vec<String>>()
            .join(","),
        emails);
    
    let name_cleaner = format!(r#"for rx in [{}]:
    print(rx, name)
    if re.search(rx, name.decode()):
        return {}[rx].encode()
return name"#,
        conf.name_substitutions.keys()
            .map(|v| format!(r#""{v}""#)).collect::<Vec<String>>()
            .join(","),
        names);
        
    


    // If not commiting, pull each repo and backup
    if !args.commit {
        info!("Processing repositories");
        // Process each repository
        for repo in conf.repositories {

            let repo_dir = repos.join(repo.clone());
            let repo_dir = repo_dir.as_path();

            let spin = ProgressBar::new_spinner()
                .with_message(format!("Processing {repo}"));
            spin.enable_steady_tick(Duration::from_millis(100));

            // Construct the repo URL
            let url = format!("git+ssh://git@github.com/{}.git", repo);

            // Load git credential options
            let mut cb = git2::RemoteCallbacks::new();
            let git_config = git2::Config::open_default().unwrap();
            let mut ch = CredentialHandler::new(git_config);
            cb.credentials(move |url, username, allowed| ch.try_next_credential(url, username, allowed));
            
            // Set fetch options
            let mut fo = git2::FetchOptions::new();
            fo.remote_callbacks(cb)
                .download_tags(git2::AutotagOption::All)
                .update_fetchhead(true);

            // Create clone dir
            std::fs::create_dir_all(repo_dir).unwrap();

            // Clone the repository
            let repository = match git2::build::RepoBuilder::new()
                    .fetch_options(fo)
                    .clone(&url, repo_dir) {
                Ok(r) => r,
                Err(e) => {

                    if e.code() != ErrorCode::Exists {
                        warn!("Received error while cloning {repo}:\n{e}");
                        warn!("Skipping cloning {url}");
                        continue;
                    }

                    // If it exists, just open the repo
                    match Repository::open(repos.join(repo.clone()).as_path()) {
                        Ok(r) => r,
                        Err(ne) => {
                            warn!("Repository {repo} already exists.");
                            warn!("Received error opening existing repo: \n{ne}");
                            warn!("Skipping cloning {url}");
                            continue;
                        },
                    }
                },
            };
            
            println!("{:?}", repository.remotes().map(|v| v.iter().filter_map(|v| v.map(|v| v.to_owned())).collect::<Vec<_>>()));

            // It's at this point that we need to drop into raw git commands, as the configuration for credential options gets waaaaay to complex
            // at this point


            info!("Fetching all branches...");

            // Fetch all branches
            Command::new("git")
                .args(["pull", "--all"])
                .current_dir(repo_dir)
                .output()
                .expect("if one git command fails, it's likely every git command will fail");
        
            
            info!("Backing up repository");

            // Create backup directory
            std::fs::create_dir_all(backups.join(repo.split('/').next().unwrap())).unwrap();

            // Backup to tar
            let backup_file = File::create(backups.join(repo.clone()).with_extension("tar"))?;
            let mut backup_tar = tar::Builder::new(backup_file);
            backup_tar.append_dir_all(".", repos.join(repo.clone()))?;
            drop(backup_tar);

            let mut branches = 0;
            // Iterate over each branch to clean out the name for each branch
            for branch in repository.branches(None).unwrap()
                .filter_map(|v| v.ok())
                .filter_map(|v| v.0.name().ok().and_then(|v| v.map(|v| v.to_string()))) {

                

                info!("Backing up branch {branch}");

                std::fs::create_dir_all(backups.join(repo.clone()).join(branch.clone()).with_extension("tar").parent().unwrap_or(Path::new(""))).unwrap();
                let backup_file = File::create(backups.join(repo.clone()).join(branch.clone()).with_extension("tar"))?;
                let mut backup_tar = tar::Builder::new(backup_file);
                backup_tar.append_dir_all(".", repos.join(repo.clone()))?;
                drop(backup_tar);


                info!("Cleaning author from branch {branch}");

                Command::new("git")
                    .args(["checkout", branch.split("/").last().unwrap()])
                    .current_dir(repo_dir)
                    .output()
                    .expect("if one git command fails, it's likely every git command will fail");

                Command::new("git")
                    .args(["filter-repo", "--force", "--partial", "--sdr", "--name-callback", &name_cleaner])
                    .current_dir(repo_dir)
                    .output()
                    .expect("if one git command fails, it's likely every git command will fail");
                    
                info!("Cleaning email from branch {branch}");
                
                Command::new("git")
                    .args(["filter-repo", "--force", "--partial", "--sdr", "--email-callback", &email_cleaner])
                    .current_dir(repo_dir)
                    .output()
                    .expect("if one git command fails, it's likely every git command will fail");
                
                
                branches += 1;
            }

            

            info!("Running garbage collection on {repo}");

            // Run git GC
            Command::new("git")
                .args(["git", "gc", "--prune=now", "--aggressive"])
                .current_dir(repo_dir)
                .output()
                .expect("if one git command fails, it's likely every git command will fail");

            if args.sign && branches == 1 {
                info!("Re-signing all commits for {repo}");
            
                Command::new("git")
                    .args(["rebase", "--exec", "git commit --amend --no-edit -n -S", "--root"])
                    .current_dir(repo_dir)
                    .output()
                    .expect("if one git command fails, it's likely every git command will fail");
            
                Command::new("git")
                    .args(["rebase", "--continue"])
                    .current_dir(repo_dir)
                    .output()
                    .expect("if one git command fails, it's likely every git command will fail");
                
                Command::new("git")
                    .args(["rebase", "--committer-date-is-author-date", "--root"])
                    .current_dir(repo_dir)
                    .output()
                    .expect("if one git command fails, it's likely every git command will fail");
            
                Command::new("git")
                    .args(["rebase", "--continue"])
                    .current_dir(repo_dir)
                    .output()
                    .expect("if one git command fails, it's likely every git command will fail");
            }

            if branches != 1 {
                warn!("Unable to re-sign history if more than one branch. Repo has {branches} branches");
            }

            spin.finish_with_message(format!("Finished processing {repo}"));
        }
    } else {
        
        info!("Force pushing every changed repository.");

        for repo in conf.repositories {

            let repo_dir = repos.join(repo.clone());
            let repo_dir = repo_dir.as_path();

            info!("Force pushing {repo}");
            
            Command::new("git")
                .args(["push", "--all", "--force"])
                .current_dir(repo_dir)
                .output()
                .expect("if one git command fails, it's likely every git command will fail");
            
            
        }
    }
    


    Ok(())
}

# Repo Cleaner


Simple tool to clean emails from the patches of a given list of git repositories

Usage: repo_cleaner.exe [ OPTIONS ] &lt;CONFIG&gt;


Arguments:\
  &lt;CONFIG&gt;  The configuration file to load from. This should be a JSON file in the following format: { "repositories": ["Org/reponame"...], "email_substitutions": { "email@example.com": "another_email@example.com" }, "name_substitutions": { "Some Name Regex": "New Name" } }

Options:    \
      --commit   Set this flag after running the command the first time. It will force push every repository that was previously modified   \
      --sign     Set this flag to sign all commits with your default GPG signing key. WARNING: This will sign *every* commit, including those not made by you! This only works on single-branch repositories \
  -h, --help     Print help \
  -V, --version  Print version  \
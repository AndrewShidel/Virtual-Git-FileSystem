use std::fs;
use chrono::{DateTime, Utc};
use std::ops::{Sub};
use time::Duration;
use std::fs::File;
use std::os::unix::fs::OpenOptionsExt;
use std::io::prelude::*;
use std::convert::TryInto;
use std::collections::{HashMap, HashSet};
use reqwest;
use std::path::Path;
use std::io;
use std::u32;
use crate::error::{GitFSError, Result};
use crate::libc_extras::libc;

struct Repo {
    // Maps a directory to a tree sha.
    tree: HashMap<String, String>,
    cloned_structures: HashSet<String>,
    timestamp_to_sha: Option<(DateTime<Utc>, String)>,
    zero_files: HashSet<String>,
}

pub struct GithubFS {
    // Maps a repo name to a repo.
    repos: HashMap<String, Repo>,
    fetched_users: HashSet<String>,
    pub token: String,
}

impl GithubFS {
    pub fn new() -> GithubFS {
        GithubFS{
            repos: HashMap::new(),
            fetched_users: HashSet::new(),
            token: "".to_string(),
        }
    }

    fn get_repo_or_create(&mut self, repo_name: &str) -> &mut Repo {
        self.repos.entry(repo_name.to_string()).or_insert_with(|| Repo{
            tree: HashMap::new(),
            cloned_structures: HashSet::new(),
            timestamp_to_sha: None,
            zero_files: HashSet::new(),
        })
    }

    // TODO: This will cause issues when multiple users have a repo of the same name.
    pub fn is_structure_cloned(&self, repo: &str, repo_dir: &str) -> bool {
        let repo_struct = self.repos.get(repo);
        if repo_struct.is_none() {
            return false;
        }
        repo_struct.unwrap().cloned_structures.contains(repo_dir)
    }

    // Returns an option which indicates if the repo exists.
    pub fn mark_as_cloned(&mut self, repo: &str, repo_file: String) {
        self.get_repo_or_create(repo).cloned_structures.insert(repo_file);
    }

    // Clones a specific directory inside of a repo, saving the empty files to the cache.
    pub fn clone_dir(&mut self, repo_dir: &str, cache_dir: &str, user: &str, repo: &str, end_time: DateTime<Utc>) -> Result<()> {
        // TODO: Do not create dirs that do not exist.
        fs::create_dir_all(cache_dir)?;
        match self.get_repo_or_create(repo).timestamp_to_sha.clone() {
            Some((_timestamp, sha)) => {
                println!("Already has timestamp");
                return self.create_fake_listing(user, repo, &sha, repo_dir, cache_dir);
            },
            // Continue on to the next match below.
            None => {},
        }
        let latest_commit = self.latest_commit_since(user, repo, end_time)?;
        self.get_repo_or_create(repo).timestamp_to_sha = Some((end_time, latest_commit.clone()));
        return self.create_fake_listing(user, repo, &latest_commit, repo_dir, cache_dir)
    }

    // TODO: Start with a recent "since" and if no commits are found work backwards to find latest.
    fn latest_commit_since(&self, user: &str, repo: &str, end_time: DateTime<Utc>) -> Result<String> {
        // TODO: This looks 10000 days into the past which is arbitrary and slow.
        let since = Utc::now().sub(Duration::days(10000));
        let endpoint = format!("repos/{}/{}/commits?since={}&until={}", user, repo, since.to_rfc3339(), end_time.to_rfc3339());
        let json = self.api_call_request(&endpoint)?;
        if !json.is_array() {
            if json.is_object() && json["message"].as_str()? == "Not Found" {
                return Err(GitFSError::new("Not Found", libc::ENOENT));
            }
            eprintln!("Invalid type for JSON result: {}", json);
            return Err(GitFSError::new("Invalid JSON", libc::EINVAL));
        }
        // JSON elements will be sorted by most recent to least recent.
        let most_recent_commit = &json.as_array()?[0];
        return Ok(most_recent_commit["sha"].as_str().map(String::from)?);
    }

    fn create_fake_listing(&mut self, user: &str, repo_name: &str, commit_sha: &str, repo_dir: &str, cache_dir: &str) -> Result<()> {
        let sha;
        {
            sha = match repo_dir {
                "" => commit_sha.to_string(),
                _ => {
                    let mut sha_result = self.get_repo_or_create(repo_name).tree.get(repo_dir);
                    if sha_result.is_none() {
                        let parent_dir = Path::new(repo_dir).parent().unwrap_or(Path::new("")).to_str()?;
                        self.create_fake_listing(user, repo_name, commit_sha, parent_dir, cache_dir)?;
                        // The directory should exist now that parent has been expanded. If it is
                        // still None then it likely does not exist.
                        sha_result = self.get_repo_or_create(repo_name).tree.get(repo_dir);
                        if sha_result.is_none() {
                            return Err(GitFSError::new("Not Found", libc::ENOENT));
                        }
                    }
                    sha_result.unwrap().clone()
                }
            };
            if self.get_repo_or_create(repo_name).zero_files.contains(repo_dir) {
                let url = format!("https://api.github.com/repos/{}/{}/git/blobs/{}", user, repo_name, sha);
                let real_path = format!("{}/{}", cache_dir, repo_dir);
                fs::create_dir_all(Path::new(&real_path).parent()?.to_str().unwrap())?;
                self.download(&url, &real_path)?;
                let repo = self.get_repo_or_create(repo_name);
                repo.zero_files.remove(repo_dir);
                repo.cloned_structures.insert(repo_dir.to_string());
                return Ok(());
            }
        }
        let tree_json = self.api_call_request(&format!("repos/{}/{}/git/trees/{}", user, repo_name, sha))?;
        
        // Check for an error message.
        let is_msg_null = tree_json["message"].is_null();
        if !is_msg_null {
            return Err(GitFSError::new(&format!("Error getting contents: {}", tree_json), libc::EIO));
        }
        
        let repo = self.get_repo_or_create(repo_name);
        repo.tree.insert(repo_dir.to_string(), tree_json["sha"].as_str()?.to_string());

        // Iterate over each entry in the directory listing.
        for node_json in tree_json["tree"].as_array()? {
            match node_json["type"].as_str() {
                // blobs are files. write empty files of the correct size as placeholders.
                Some("blob") => {
                    let path = Path::new(repo_dir).join(node_json["path"].as_str()?);
                    if repo.cloned_structures.contains(path.to_str()?) {
                        println!("Skipping already cloned file: {}", path.to_str()?);
                        continue;
                    }
                    let real_path = Path::new(cache_dir).join(path.as_path());
                    // TODO: Use node_json["mode"].as_str() here.
                    let mut file = fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .mode(u32::from_str_radix(node_json["mode"].as_str()?, 8).unwrap())
                        .open(real_path.as_path())?;
                    let f_size = node_json["size"].as_i64()?;
                    file.write_all(&vec![0; f_size.try_into().unwrap()])?;
                    repo.zero_files.insert(path.to_str()?.to_string());
                    repo.tree.insert(path.to_str()?.to_string(), node_json["sha"].as_str()?.to_string());
                },
                // Trees are directories. Simply create an empty directory.
                Some("tree") => {
                    let tree_sha = node_json["sha"].as_str()?.to_string();
                    let path = Path::new(repo_dir).join(node_json["path"].as_str()?);
                    repo.tree.insert(path.to_str()?.to_string(), tree_sha);
                    // TODO: Use node_json["mode"].as_str() here.
                    fs::create_dir_all(format!("{}/{}", cache_dir, path.to_str()?))?;
                },
                _ => {
                    eprintln!("Unknown type: {}", node_json["type"])
                }
            }
        }
        // Create an empty .git directory. The contents will only be created when a file within
        // this directory is accessed.
        fs::create_dir_all(format!("{}/.git", cache_dir))?;
        repo.cloned_structures.insert(repo_dir.to_string());
        Ok(())
    }

    fn user_info(&self, user: &str) -> Result<serde_json::value::Value> {
        let repos_endpoint = format!("users/{}/repos", user);
        self.api_call_request(&repos_endpoint)
    }

    // Creates the repo directories in the cache for a given user.
    // TODO: Filter out repos created after sync time.
    pub fn fill_user_repos(&mut self, path: &str, user: &str) -> Result<()> {
        if self.fetched_users.contains(user) {
            return Ok(())
        }
        let json = self.user_info(user)?;
        self.fetched_users.insert(user.to_string());
        if json.as_array()?.len() > 0 {
            fs::create_dir(&path)?;
        }
        for e in json.as_array()? {
            let name = e["name"].as_str()?;
            fs::create_dir(format!("{}/{}", path, name))?;
        }
        Ok(())
    }

    fn api_call_request(&self, endpoint: &str) -> Result<serde_json::value::Value> {
        let url = format!("https://api.github.com/{}", &endpoint);
        println!("Request {}", url);
        let client = reqwest::blocking::Client::new();
        let res = client.get(&url).header(reqwest::header::USER_AGENT, "Virtual Git Filesystem").header("Authorization", format!("token {}", self.token)).send()?;
        res.error_for_status_ref()?;
        let json_str = res.text()?;
        match serde_json::from_str(&json_str) {
            Ok(json) => Ok(json),
            Err(e) => {
                eprintln!("Unable to parse JSON: {}", e);
                Err(GitFSError::new("Unable to parse JSON", libc::EINVAL))
            }
        }
    }

    fn download(&self, remote_path: &str, local_path: &str) -> Result<()> {
        let client = reqwest::blocking::Client::new();
        let mut resp = client.get(remote_path)
            .header(reqwest::header::USER_AGENT, "Virtual Git Filesystem")
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github.VERSION.raw")
            .send()?;
        resp.error_for_status_ref()?;
        let mut out = File::create(local_path)?;
        io::copy(&mut resp, &mut out)?;
        Ok(())
    }
}

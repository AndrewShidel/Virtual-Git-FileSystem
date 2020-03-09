use std::fs;
use chrono::{DateTime, Utc};
use std::ops::{Sub};
use time::Duration;
use std::fs::File;
use std::io::prelude::*;
use std::convert::TryInto;
use std::collections::{HashMap, HashSet};
use reqwest;
use std::path::Path;
use std::io;
use crate::error::{GitFSError, Result};
use crate::libc_extras::libc;


fn download(remote_path: &str, local_path: &str) -> Result<()> {
    let mut resp = reqwest::blocking::get(remote_path)?;
    let mut out = File::create(local_path)?;
    io::copy(&mut resp, &mut out)?;
    Ok(())
}


struct Repo {
    // Maps a (directory, commit sha) to a tree sha.
    tree: HashMap<(String, String), String>,
    cloned_structures: HashSet<String>,
    timestamp_to_sha: Option<(DateTime<Utc>, String)>,
}

pub struct GithubFS {
    // Maps a repo name to a repo.
    repos: HashMap<String, Repo>,
    fetched_users: HashSet<String>,
    pub token: String,
}

impl GithubFS {
    pub fn new() -> GithubFS {
        GithubFS{repos: HashMap::new(), fetched_users: HashSet::new(), token: "".to_string()}
    }

    fn get_repo_or_create(&mut self, repo_name: &str) -> &mut Repo {
        self.repos.entry(repo_name.to_string()).or_insert_with(|| Repo{
            tree: HashMap::new(),
            cloned_structures: HashSet::new(),
            timestamp_to_sha: None,
        })
    }

    pub fn is_structure_cloned(&self, repo: &str, repo_dir: &str) -> bool {
        let repo_struct = self.repos.get(repo);
        if repo_struct.is_none() {
            return false;
        }
        repo_struct.unwrap().cloned_structures.contains(repo_dir)
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
        // Note: This will only get the root of the repo.
        let tree_json = self.api_call_request(&format!("repos/{}/{}/contents/{}?ref={}", user, repo_name, repo_dir, commit_sha))?;
        let repo = self.get_repo_or_create(repo_name);
        if tree_json.is_object() {
            let is_msg_null = tree_json["message"].is_null();
            if !is_msg_null {
                return Err(GitFSError::new(&format!("Error getting contents: {}", tree_json), libc::EIO));
            }
            let path_str = format!("{}/{}", cache_dir, tree_json["path"].as_str()?);
            let path = Path::new(&path_str);
            fs::create_dir_all(path.parent()?.to_str().unwrap())?;
            //let mut file = File::create(&path_str).unwrap();
            //file.write_all(&base64::decode(tree_json["content"].as_str().unwrap()).unwrap());
            let download_url = tree_json["download_url"].as_str()?;
            download(download_url, &path_str)?;
            println!("Downloaded file {} to {}", download_url, &path_str);
            repo.cloned_structures.insert(tree_json["path"].as_str()?.to_string());
            return Ok(());
        }
        // Handle Directory
        for node_json in tree_json.as_array()? {
            match node_json["type"].as_str() {
                Some("file") => {
                    let path = node_json["path"].as_str()?;
                    if repo.cloned_structures.contains(path) {
                        println!("Skipping already cloned file: {}", path);
                        continue;
                    }
                    let mut file = File::create(format!("{}/{}", cache_dir, node_json["path"].as_str()?))?;
                    let f_size = node_json["size"].as_i64()?;
                    //file.write_all(&[0u8; node_json["size"].as_i64()?]);
                    file.write_all(&vec![0; f_size.try_into().unwrap()])?;
                },
                Some("dir") => {
                    let tree_sha = node_json["sha"].as_str()?.to_string();
                    repo.tree.insert((repo_dir.to_string(), commit_sha.to_string()), tree_sha);
                    fs::create_dir_all(format!("{}/{}", cache_dir, node_json["path"].as_str()?))?;
                },
                _ => {
                    eprintln!("Unknown type: {}", node_json["type"])
                }
            }
        }
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
}

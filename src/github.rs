use serde_json::Value;
use std::borrow::{Borrow, BorrowMut};
use std::fs;
use chrono::{DateTime, Utc};
use std::ops::{Sub, Deref};
use time::Duration;
use std::fs::File;
use std::io::prelude::*;
use std::convert::TryInto;
use std::collections::{HashMap, HashSet};
use reqwest;
use reqwest::Client;
use std::error;
use std::hash::Hash;
use std::path::Path;
use base64;
use std::io;
use http;

fn download(remote_path: &str, local_path: &str) -> Result<(), Box<std::error::Error>> {
    let mut resp = reqwest::blocking::get(remote_path)?;
    let mut out = File::create(local_path)?;
    io::copy(&mut resp, &mut out)?;
    Ok(())
}


struct Repo {
    // Maps a (directory, commit sha) to a tree sha.
    tree: HashMap<(String, String), String>,
    clonedStructures: HashSet<String>,
    timestamp_to_sha: Option<(DateTime<Utc>, String)>,
}

pub struct GithubFS {
    // Maps a repo name to a repo.
    repos: HashMap<String, Repo>,
    pub token: String,
}

impl GithubFS {
    pub fn new() -> GithubFS {
        GithubFS{repos: HashMap::new(), token: "".to_string()}
    }

    fn get_repo_or_create(&mut self, repo_name: &str) -> &mut Repo {
        self.repos.entry(repo_name.to_string()).or_insert_with(|| Repo{
            tree: HashMap::new(),
            clonedStructures: HashSet::new(),
            timestamp_to_sha: None,
        })
    }

    pub fn is_structure_cloned(&self, repo: &str, repo_dir: &str) -> bool {
        let repoStruct = self.repos.get(repo);
        if repoStruct.is_none() {
            return false;
        }
        repoStruct.unwrap().clonedStructures.contains(repo_dir)
    }

    // Clones a specific directory inside of a repo, saving the empty files to the cache.
    pub fn clone_dir(&mut self, repo_dir: &str, cache_dir: &str, user: &str, repo: &str, end_time: DateTime<Utc>) {
        // TODO: Do not create dirs that do not exist.
        fs::create_dir_all(cache_dir);
        match self.get_repo_or_create(repo).timestamp_to_sha.clone() {
            Some((timestamp, sha)) => {
                println!("Already has timestamp");
                self.create_fake_listing(user, repo, &sha, repo_dir, cache_dir);
                return
            },
            None => {},
        }
        println!("Getting timestamp");
        match self.latest_commit_since(user, repo, end_time) {
            Some(latest_commit) => {
                self.get_repo_or_create(repo).timestamp_to_sha = Some((end_time, latest_commit.clone()));
                self.create_fake_listing(user, repo, &latest_commit, repo_dir, cache_dir);
            },
            None => {
                eprintln!("Could not find latest commit since: user={}, repo={}, end_time={}", user, repo, end_time);
                return
            },
        }
    }

    // TODO: Start with a recent "since" and if no commits are found work backwards to find latest.
    fn latest_commit_since(&self, user: &str, repo: &str, end_time: DateTime<Utc>) -> Option<String> {
        let since = Utc::now().checked_sub_signed(Duration::days(10000)).unwrap();
        let endpoint = format!("repos/{}/{}/commits?since={}&until={}", user, repo, since.to_rfc3339(), end_time.to_rfc3339());
        let res = self.api_call_request(&endpoint);
        if res.is_none() {
            return None
        }
        let json = res.unwrap();
        // JSON elements will be sorted by most recent to least recent.
        let most_recent_commit = &json.as_array().unwrap()[0];
        //return most_recent_commit["commit"]["tree"]["sha"].as_str().map(String::from);
        return most_recent_commit["sha"].as_str().map(String::from);
    }

    fn create_fake_listing(&mut self, user: &str, repo_name: &str, commit_sha: &str, repo_dir: &str, cache_dir: &str) {
        // Note: This will only get the root of the repo.
        let res = self.api_call_request(&format!("repos/{}/{}/contents/{}?ref={}", user, repo_name, repo_dir, commit_sha));
        let tree_json = res.unwrap();
        //if let Some(tree_json) = res {
        println!("tree_json: {}", tree_json);
        let mut repo = self.get_repo_or_create(repo_name);
        if tree_json.is_object() {
            let is_msg_null = tree_json["message"].is_null();
            if !is_msg_null {
                eprintln!("Error getting contents: {}", tree_json);
                return
            }
            let path_str = format!("{}/{}", cache_dir, tree_json["path"].as_str().unwrap());
            let path = Path::new(&path_str);
            fs::create_dir_all(path.parent().unwrap().to_str().unwrap());
            //let mut file = File::create(&path_str).unwrap();
            //file.write_all(&base64::decode(tree_json["content"].as_str().unwrap()).unwrap());
            let download_url = tree_json["download_url"].as_str().unwrap();
            match download(download_url, &path_str) {
                Ok(_) => {
                    println!("    Downloaded file {} to {}", download_url, &path_str);
                    repo.clonedStructures.insert(repo_dir.to_string());
                },
                Err(e) => {
                    println!("Error downloading file: {}", e);
                },
            }
            return
        }
        // Handle Directory
        for node_json in tree_json.as_array().unwrap() {
            println!("tree element:{}", node_json);
            match node_json["type"].as_str() {
                Some("file") => {
                    println!("{}/{}", cache_dir, node_json["path"].as_str().unwrap());
                    let mut file = File::create(format!("{}/{}", cache_dir, node_json["path"].as_str().unwrap())).unwrap();
                    let f_size = node_json["size"].as_i64().unwrap();
                    //file.write_all(&[0u8; node_json["size"].as_i64().unwrap()]);
                    file.write_all(&vec![0; f_size.try_into().unwrap()]);
                },
                Some("dir") => {
                    let tree_sha = node_json["sha"].as_str().unwrap().to_string();
                    repo.tree.insert((repo_dir.to_string(), commit_sha.to_string()), tree_sha);
                    fs::create_dir_all(format!("{}/{}", cache_dir, node_json["path"].as_str().unwrap()));
                    repo.clonedStructures.insert(repo_dir.to_string());
                },
                _ => {
                    println!("Unknown type: {}", node_json["type"])
                }
            }
        }
        //} else if let Err(e) = res {
        //    println!("Error getting directory or file info: {}", e);
        //    return
        //}
    }

    fn user_info(&self, user: &str) -> Option<serde_json::value::Value> {
        let repos_endpoint = format!("users/{}/repos", user);
        return self.api_call_request(&repos_endpoint);
    }

    // Creates the repo directories in the cache for a given user.
    // TODO: Filter out repos created after sync time.
    pub fn fill_user_repos(&self, path: &str, user: &str) {
        let info = self.user_info(user);
        if info.is_none() {
            return;
        }
        let json = info.unwrap();
        if json.is_array() {
            println!("Is an array");
        } else if json.is_object() {
            println!("Is an object");
        } else {
            println!("The type is unknown");
            std::process::exit(1);
        }
        for e in json.as_array().unwrap() {
            let name = e["name"].as_str().unwrap();
            println!("Name: {}", name);
            fs::create_dir(format!("{}/{}", path, name));
        }
    }

    fn api_call_request(&self, endpoint: &str) -> Option<serde_json::value::Value> {
        let url = format!("https://api.github.com/{}", &endpoint) ;
        println!("Request {}", url);
        let client = reqwest::blocking::Client::new();
        let response = client.get(&url).header(reqwest::header::USER_AGENT, "Virtual Git Filesystem").header("Authorization", format!("token {}", self.token)).send();
        //let response = reqwest::blocking::get(&url);
        match response {
            Ok(res) => {
                let json_str = res.text().unwrap();
                match serde_json::from_str(&json_str) {
                    Ok(json) => {
                        Some(json)
                    },
                    Err(e) => {
                        eprintln!("Error parsing JSON for {}: {}, JSON={}", endpoint, e, &json_str);
                        None
                    }
                }
            },
            Err(e) => {
                eprintln!("API error for {}: {}", endpoint, e);
                None
            },
        }
    }
}

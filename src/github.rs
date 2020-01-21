use github_rs::client::{Executor, Github};
use serde_json::Value;
use std::borrow::Borrow;
use std::fs;
use chrono::{DateTime, Utc};
use std::ops::Sub;
use time::Duration;

fn api_call(endpoint: &str) -> Option<serde_json::value::Value> {
    let client = Github::new("e6bc4bdc7e065da2041510946d921ac961094f3d").unwrap();
    let response = client
        .get()
        .custom_endpoint(&endpoint)
        .execute::<Value>();
    match response {
        Ok((_, status, json)) => {
            println!("Status {}", status);
            if let Some(json) = &json {
                println!("{}", json);
            }
            if !status.is_success() {
                println!("Status is not success");
                return None;
            }
            return json;
        },
        Err(e) => {
            println!("GitHub Error {}", e);
            return None;
        }
    }
}

pub fn latest_commit_since(user: &str, repo: &str, end_time: DateTime<Utc>) {
    let since = Utc::now().checked_sub_signed(Duration::days(10)).unwrap();
    let endpoint = format!("repos/{}/{}/commits?since={}&until={}", user, repo, since.to_rfc3339(), end_time.to_rfc3339());
    if let Some(json) = api_call(&endpoint) {
        println!("latest_commit_since: {}", json);
        // JSON elements will be sorted by most recent to least recent.
        let most_recent_commit = &json.as_array().unwrap()[0];
        let sha = most_recent_commit["commit"]["tree"]["sha"].as_str().unwrap();
        create_fake_listing(user, repo, sha);
    }
}

pub fn create_fake_listing(user: &str, repo: &str, commit_sha: &str) {
    if let Some(tree_json) = api_call(&format!("repos/{}/{}/git/trees/{}", user, repo, sha)) {
        for node_json in tree_json["tree"].as_array().unwrap() {
            println!("tree element:{}", node_json);
        }
    }
}

pub fn dir_info(user: &str, repo: &str, path: &str) -> Option<serde_json::value::Value> {
    // TODO: Share client between API calls.
    let endpoint = format!("repos/{}/{}/contents/{}", user, repo, path);
    return api_call(&endpoint);
}

pub fn user_info(user: &str) -> Option<serde_json::value::Value> {
    let repos_endpoint = format!("users/{}/repos", user);
    return api_call(&repos_endpoint);
}

pub fn fill_user_repos(path: &str, user: &str) {
    let info = user_info(user);
    if info.is_none() {
        return;
    }
    if let Some(json) = info {
        println!("JSON: {}", json);
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
}

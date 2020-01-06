use github_rs::client::{Executor, Github};
use serde_json::Value;
use std::borrow::Borrow;
use std::fs;

pub fn user_info(user: &str) -> Option<serde_json::value::Value> {
    let client = Github::new("e6bc4bdc7e065da2041510946d921ac961094f3d").unwrap();
    //let me = client.get()
    //               .user()
    //               .execute::<Value>();
    let repos_endpoint = format!("users/{}/repos", user);
    //execute
    let response = client
        .get()
        //set custom endpoint here
        .custom_endpoint(&repos_endpoint)
        .execute::<Value>();
    //client.get().custom_endpoint().execute::<Value>()
    match response {
        Ok((headers, status, json)) => {
            //println!("{:#?}", headers);
            println!("Status {}", status);
            //if let Some(json) = json{
            //    println!("{}", json);
            //}
            if !status.is_success() {
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

pub fn fill_user_repos(user: &str) {
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
            fs::create_dir(format!("/tmp/users/{}/{}", user, name));
        }
    }
}

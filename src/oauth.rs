use dirs;
use rouille;
use webbrowser;
use reqwest;
use std::fs;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::sync::Mutex;

fn get_redirect_code() -> String {
    // TODO: start using server struct run and call server.poll() in a loop until a code is found.
    let (code_sender, code_receiver): (Sender<String>, Receiver<String>) = mpsc::channel();
    let sender_wrapped: Mutex<Sender<String>> = Mutex::new(code_sender);
    // 35918 is the port specified for the Github OAuth callback. Generated with RNG.
    let server = rouille::Server::new("localhost:35918", move |request| {
        router!(request,
            (GET) (/) => {
                let code = request.get_param("code").unwrap();
                println!("Sending code: {}", &code);
                sender_wrapped.lock().unwrap().send(code).unwrap();
                println!("Code sent");
                rouille::Response::text(format!("You are now authenticated and can use Virtual Git Filesystem."))
            },

            // The code block is called if none of the other blocks matches the request.
            // We return an empty response with a 404 status code.
            _ => rouille::Response::empty_404()
        )
    }).unwrap();
    launch_webpage();
    loop {
        match code_receiver.try_recv() {
            Ok(code) => {
                println!("Got code: {}", &code);
                return code;
            },
            Err(_) => {},
        }
        server.poll();
    }
}

fn exchange_for_token(code: String) -> String {
    let client = reqwest::blocking::Client::new();
    let res: serde_json::value::Value = client.post(&format!("https://github.com/login/oauth/access_token?client_id={}&client_secret={}&code={}", "d8dfe8c41abaf9d989a6", "eeb6bcc04cd11210cd4554a02c4ddf740a28d5df", code))
        .header("Accept", "application/json")
        .send().unwrap().json().unwrap();
    res["access_token"].as_str().unwrap().to_string()
}

fn launch_webpage() {
    let url = format!("https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope={}&state={}", "d8dfe8c41abaf9d989a6", "http://localhost:35918", "repo", "changethis");
    if !webbrowser::open(&url).is_ok() {
        eprintln!("Unable to open web browser for OAuth exchange. Some features will be limited.")
    }
}

pub fn get_token() -> Option<String> {
    let cache_dir = dirs::cache_dir().unwrap();
    let credential_dir = format!("{}/gitfs/.credentials", cache_dir.to_str().unwrap());
    std::fs::create_dir_all(&credential_dir).unwrap();
    let token_file = format!("{}/.token", credential_dir);
    match std::fs::read_to_string(&token_file) {
        Ok(token) => {
            return Some(token);
        },
        Err(_) => {
            println!("No Github token found. Starting OAuth flow.")
        }
    }
    let code = get_redirect_code();
    println!("Code was returned: {}", &code);
    let token = exchange_for_token(code);
    fs::write(token_file, &token).unwrap();
    Some(token)
}

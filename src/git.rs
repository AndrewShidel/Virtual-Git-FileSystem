
use git2::build::{RepoBuilder};
use std::path::{Path};

pub fn clone_if_not_exist(repo_path: &String, cache_dir: String) -> Result<String, std::io::Error> {
    let parts: Vec<&str> = repo_path.split("/").collect();
    if parts.len() < 3 {
        // TODO(ashidel): Return error
        return Ok("".to_string())
    }
    let url = "https://".to_owned() + parts[0..2].join("/").as_str() + ".git";
    debug!("Final URL: {:?}", url);
    match RepoBuilder::new().clone(&url, Path::new(&cache_dir)) {
        Ok(_r) => Ok(cache_dir),
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::NotFound, e))
    }
}
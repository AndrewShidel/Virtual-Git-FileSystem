extern crate tempdir;

use git2::build::{RepoBuilder};
use std::path::{Path};
use tempdir::TempDir;
use crate::github;
use std::fs;
use chrono::Utc;

pub fn clone_if_not_exist(repo_path: String, cache_dir: String, ignoreBase: bool) -> Result<String, std::io::Error> {
    let parts: Vec<&str> = repo_path.split("/").collect();
    println!("repo_path: {}, parts: {:?}", repo_path, parts);
    if parts.len() == 1 {
        if parts[0] != "" {
            return Ok(format!("{}/repos/github.com", cache_dir));
        } else {
            return Ok(format!("{}/repos", cache_dir));
        }
    }
    if parts.len() > 0 && parts[0] != "github.com" {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Not Found"));
    }
    if parts.len() == 2 {
        let path = format!("{}/repos/github.com/{}", cache_dir, parts[1]);
        if !Path::new(&path).exists() {
            fs::create_dir(&path);
            github::fill_user_repos(&path, parts[1]);
        }
        return Ok(path);
    }
    if parts.len() < 3 {
        return Ok("/".to_string())
    }
    
    // real_repo_path is the location of the repo in the real local filesystem.
    let real_repo_path = format!("{}/github.com/repos/{}/{}", cache_dir, parts[1], parts[2]);
    let real_file_path = format!("{}/github.com/repos/{}", cache_dir, parts[1..].join("/").as_str());
    
    // If this is only the full repo path and the base path is being ignored, then do not clone the
    // repo.
    if ignoreBase && parts.len() == 3 {
        // Give it a temporary directory so that it can display metadata for the directory without
        // the need to create the real one.
        let tmpPath = format!("{}/fake_repos/github.com/{}/{}", cache_dir, parts[1], parts[2]);
        fs::create_dir_all(tmpPath.clone());
        return Ok(tmpPath);
    }

    let url = "https://".to_owned() + parts[0..3].join("/").as_str() + ".git";
    println!("Final URL: {:?}", url);
    println!("Clonning to: {}", real_repo_path);
    if Path::new(&real_repo_path).exists() {
        return Ok(real_file_path)
    }
    github::latest_commit_since(parts[1], parts[2], Utc::now());
    match RepoBuilder::new().clone(&url, Path::new(&real_repo_path)) {
        Ok(_r) => Ok(real_file_path),
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::NotFound, e))
    }
}

//TODO: Mock out clone step for tests.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clone_root() -> Result<(), std::io::Error> {
        let path = clone_if_not_exist(&"".to_string(), "/tmp/cache".to_string())?;
        assert_eq!(path, "/");
        Ok(())
    }

    #[test]
    fn test_clone_full() -> Result<(), std::io::Error> {
        let tmp_dir = TempDir::new("cache")?;
        let path = clone_if_not_exist(&"github.com/AndrewShidel/Resume".to_string(), tmp_dir.path().to_str().unwrap().to_string())?;
        assert_eq!(path, tmp_dir.path().join("Resume").to_str().unwrap());
        Ok(())
    }

    #[test]
    fn test_clone_extras() -> Result<(), std::io::Error> {
        let tmp_dir = TempDir::new("cache")?;
        let path = clone_if_not_exist(&"github.com/AndrewShidel/Resume/resume.tex".to_string(), tmp_dir.path().to_str().unwrap().to_string())?;
        assert_eq!(path, format!("{}/Resume/resume.tex", tmp_dir.path().to_str().unwrap()));
        Ok(())
    }
}

extern crate tempdir;

use git2::build::{RepoBuilder};
use std::path::{Path};
use tempdir::TempDir;
use crate::github::{GithubFS};
use std::fs;
use chrono::Utc;

pub struct GitFS {
    github: GithubFS,
}

impl GitFS {
    pub fn new() -> GitFS {
        GitFS{github: GithubFS::new()}
    }
    pub fn clone_if_not_exist(&mut self, repo_path: String, cache_dir: String, ignore_base: bool, is_stat: bool) -> Result<String, std::io::Error> {
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
                self.github.fill_user_repos(&path, parts[1]);
            }
            return Ok(path);
        }
        if parts.len() < 3 {
            return Ok("/".to_string())
        }

        // real_repo_path is the location of the repo in the real local filesystem.
        let real_repo_path = format!("{}/github.com/repos/{}/{}", cache_dir, parts[1], parts[2]);
        let path_in_repo = parts[3..].join("/");
        let real_file_path = format!("{}/github.com/repos/{}", cache_dir, parts[1..].join("/").as_str());

        // If this is only the full repo path and the base path is being ignored, then do not clone the
        // repo.
        if ignore_base && parts.len() == 3 {
            // Give it a temporary directory so that it can display metadata for the directory without
            // the need to create the real one.
            let tmpPath = format!("{}/fake_repos/github.com/{}/{}", cache_dir, parts[1], parts[2]);
            fs::create_dir_all(tmpPath.clone());
            return Ok(tmpPath);
        }

        let url = "https://".to_owned() + parts[0..3].join("/").as_str() + ".git";
        println!("Final URL: {:?}", url);
        // If all we need is metadata about the file/directory, then it is sufficient to just clone the parent directory.
        if is_stat {
            let repo_parent = Path::new(&path_in_repo).parent().unwrap_or(Path::new("/")).to_str().unwrap();
            if !self.github.is_structure_cloned(parts[2], repo_parent) {
                self.github.clone_dir(repo_parent, &real_repo_path, parts[1], parts[2], Utc::now());
            }
            return Ok(real_file_path)
        }
        if self.github.is_structure_cloned(parts[2], &path_in_repo) {
            return Ok(real_file_path)
        }
        // TODO: If the path is in ".git" discard existing cache and do a fresh clone.
        self.github.clone_dir(&path_in_repo, &real_repo_path, parts[1], parts[2], Utc::now());
        Ok(real_file_path)
        //match RepoBuilder::new().clone(&url, Path::new(&real_repo_path)) {
        //    Ok(_r) => Ok(real_file_path),
        //    Err(e) => Err(std::io::Error::new(std::io::ErrorKind::NotFound, e))
        //}
    }
}

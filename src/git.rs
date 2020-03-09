extern crate tempdir;

// TODO: Uncomment this when adding support for non-Github repos.
//use git2::build::{RepoBuilder};
use std::path::{Path};
use crate::github::{GithubFS};
use chrono::{DateTime, Utc};
use std::fs;
use crate::error::{Result, GitFSError};
use crate::libc_extras::libc;

pub struct GitFS {
    github: GithubFS,
    timestamp: DateTime<Utc>,
}

impl GitFS {
    pub fn new() -> GitFS {
        GitFS{
            github: GithubFS::new(),
            timestamp: Utc::now(),
        }
    }

    pub fn set_token(&mut self, token: String) {
        self.github.token = token;
    }

    pub fn clone_if_not_exist(&mut self, repo_path: String, cache_dir: String, ignore_base: bool, is_stat: bool) -> Result<String> {
        let parts: Vec<&str> = repo_path.split("/").collect();
        println!("repo_path: {}, parts: {:?}", repo_path, parts);
        // For now we only support github, so all other domains should fail.
        if parts[0] != "github.com" && parts[0] != "" {
            return Err(GitFSError::new("Not Found", libc::ENOENT))
        }
        if parts.len() == 1 {
            if parts[0] != "" {
                return Ok(format!("{}/repos/github.com", cache_dir));
            } else {
                return Ok(format!("{}/repos", cache_dir));
            }
        }
        if parts.len() == 2 {
            match parts[1] {
                // These are all common metadata files/directories. Do not try to look them up as
                // users for performance reasons and because some of these resolve to valid Github 
                // users.
                // TODO: There are many more of these which can be filtered.
                "HEAD"|".git"|"BUILD"|"WORKSPACE"|".idea" => {
                    return Err(GitFSError::new("Not Found", libc::ENOENT));
                },
                _ => {},
            }
            let path = format!("{}/repos/github.com/{}", cache_dir, parts[1]);
            if !Path::new(&path).exists() {
                self.github.fill_user_repos(&path, parts[1])?;
            }
            return Ok(path);
        }
        if parts.len() < 3 {
            return Ok("/".to_string())
        }

        // real_repo_path is the location of the repo in the real local filesystem.
        let real_repo_path = format!("{}/repos/github.com/{}/{}", cache_dir, parts[1], parts[2]);
        let path_in_repo = parts[3..].join("/");
        let real_file_path = format!("{}/repos/github.com/{}", cache_dir, parts[1..].join("/").as_str());

        // If this is only the full repo path and the base path is being ignored, then do not clone the
        // repo.
        if ignore_base && parts.len() == 3 {
            let path = format!("{}/repos/github.com/{}", cache_dir, parts[1]);
            self.github.fill_user_repos(&path, parts[1])?;
            if Path::new(&real_repo_path).exists() {
                return Ok(real_repo_path);
            }
            println!("DOES NOT EXIST {}", &real_repo_path);
            return Err(GitFSError::new("Not Found", libc::ENOENT));
        }

        let url = "https://".to_owned() + parts[0..3].join("/").as_str() + ".git";
        println!("Final Repo URL: {:?}", url);
        // If all we need is metadata about the file/directory, then it is sufficient to just clone the parent directory.
        if is_stat {
            let repo_parent = Path::new(&path_in_repo).parent().unwrap_or(Path::new("/")).to_str()?;
            if !self.github.is_structure_cloned(parts[2], repo_parent) {
                self.github.clone_dir(repo_parent, &real_repo_path, parts[1], parts[2], self.timestamp)?;
            }
            return Ok(real_file_path)
        }
        if self.github.is_structure_cloned(parts[2], &path_in_repo) {
            return Ok(real_file_path)
        }
        // TODO: If the path is in ".git" discard existing cache and do a fresh clone.
        self.github.clone_dir(&path_in_repo, &real_repo_path, parts[1], parts[2], self.timestamp)?;
        Ok(real_file_path)
        //match RepoBuilder::new().clone(&url, Path::new(&real_repo_path)) {
        //    Ok(_r) => Ok(real_file_path),
        //    Err(e) => Err(std::io::Error::new(std::io::ErrorKind::NotFound, e))
        //}
    }
}

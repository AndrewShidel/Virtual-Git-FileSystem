extern crate tempdir;

// TODO: Uncomment this when adding support for non-Github repos.
//use git2::build::{RepoBuilder};
use std::path::{Path};
use std::process::Command;
use crate::github::{GithubFS};
use chrono::{DateTime, Utc};
use std::fs;
use crate::error::{Result, GitFSError};
use crate::libc_extras::libc;
use std::collections::{HashSet};
use walkdir::WalkDir;

pub struct GitFS {
    github: GithubFS,
    timestamp: DateTime<Utc>,
    // The set of all git URLs which have been cloned using full_clone.
    fully_cloned_paths: HashSet<String>,
    cache_dir: String,
}

impl GitFS {
    pub fn new() -> GitFS {
        GitFS{
            github: GithubFS::new(),
            timestamp: Utc::now(),
            fully_cloned_paths: HashSet::new(),
            // This will be filled in later by set_cache_dir.
            cache_dir: "/dev/null".to_string(),
        }
    }

    pub fn set_token(&mut self, token: String) {
        self.github.token = token;
    }

    pub fn set_cache_dir(&mut self, cache_dir: String) {
        self.cache_dir = cache_dir;
    }

    pub fn clone_if_not_exist(&mut self, repo_path: String, ignore_base: bool, is_stat: bool) -> Result<String> {
        let cache_dir = self.cache_dir.clone();
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

        // If the path is in the .git directory, clone if needed then return the path to the real
        // file.
        if parts.len() > 3 && parts[3] == ".git" {
            // If it is simply a stat of the .git directory, just return the path to the empty
            // file.
            if is_stat && parts.len() == 4 {
                fs::create_dir_all(&real_file_path)?;
                return Ok(real_file_path);
            }
            self.full_clone(parts[1], parts[2], &url, &cache_dir, &real_repo_path)?;
            return Ok(real_file_path);
        }

        // If all we need is metadata about the file/directory, then it is sufficient to just clone the parent directory.
        if is_stat {
            let repo_parent = Path::new(&path_in_repo).parent().unwrap_or(Path::new("/")).to_str()?;
            if self.github.is_structure_cloned(parts[2], repo_parent) || self.github.is_structure_cloned(parts[2], &path_in_repo) {
                return Ok(real_file_path)
            }
            self.github.clone_dir(repo_parent, &real_repo_path, parts[1], parts[2], self.timestamp)?;
            return Ok(real_file_path)
        }
        if self.github.is_structure_cloned(parts[2], &path_in_repo) {
            return Ok(real_file_path)
        }
        self.github.clone_dir(&path_in_repo, &real_repo_path, parts[1], parts[2], self.timestamp)?;
        Ok(real_file_path)
    }

    fn full_clone(&mut self, user: &str, repo: &str, url: &str, cache_dir: &str, repo_path: &str) -> Result<()> {
        let repo_clone_dir = format!("{}/tmp_repos/{}/{}", cache_dir, user, repo);
        if self.fully_cloned_paths.contains(url) {
            return Ok(());
        }
        let mut child = Command::new("git")
            .arg("clone")
            //.arg("--no-checkout")
            .arg(url)
            .arg(&repo_clone_dir)
            .spawn()?;
        child.wait()?;
        let tmp_repo_git_dir = format!("{}/.git", &repo_clone_dir);
        let real_repo_git_dir = format!("{}/.git", repo_path);
        fs::rename(tmp_repo_git_dir, real_repo_git_dir)?;
        for entry_result in WalkDir::new(&repo_clone_dir) {
            let entry = entry_result.unwrap(); // TODO: Do not use unwrap.
            let entry_path = entry.path();
            let relative_path = entry_path.strip_prefix(&repo_clone_dir).unwrap_or(entry_path).to_str()?;
            println!("path = {}, relative_path = {}", entry_path.display(), relative_path);
            if !self.github.is_structure_cloned(repo, relative_path) {
                if entry.file_type().is_dir() {
                    fs::create_dir_all(format!("{}/{}", repo_path, relative_path))?;
                } else {
                    fs::create_dir_all(Path::new(&format!("{}/{}", repo_path, relative_path)).parent()?)?;
                    println!("Copying path = {}, relative_path = {}", entry_path.display(), relative_path);
                    fs::rename(entry_path, format!("{}/{}", repo_path, relative_path))?;
                }
                // Mark the file as cached.
                self.github.mark_as_cloned(repo,  relative_path.to_string());
            } else {
                println!("NOT copying path = {}, relative_path = {}", entry_path.display(), relative_path);
            }
        }
        println!("Everything was copied.");
        self.fully_cloned_paths.insert(url.to_string());
        return Ok(());
    }
}

extern crate tempdir;

use git2::build::{RepoBuilder};
use std::path::{Path};
use tempdir::TempDir;

pub fn clone_if_not_exist(repo_path: String, cache_dir: String) -> Result<String, std::io::Error> {
    let parts: Vec<&str> = repo_path.split("/").collect();
    println!("repo_path: {}, parts: {:?}", repo_path, parts);
    if parts.len() < 3 {
        return Ok("/".to_string())
    }
    let url = "https://".to_owned() + parts[0..3].join("/").as_str() + ".git";
    let real_repo_path = format!("{}/{}", cache_dir, parts[2]);
    if Path::new(&real_repo_path).exists() {
        return Ok(format!("{}/{}", cache_dir, parts[2..].join("/").as_str()))
    }
    println!("Final URL: {:?}", url);
    match RepoBuilder::new().clone(&url, Path::new(&real_repo_path)) {
        Ok(_r) => Ok(format!("{}/{}", cache_dir, parts[2..].join("/").as_str())),
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

use std::error;
use std::fmt;
use reqwest;
use reqwest::{StatusCode};

pub type Result<T> = std::result::Result<T, GitFSError>;

#[derive(Debug)]
pub struct InternalError {
    code: i32,
    msg: String,
}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "msg = {}, code = {}", self.msg, self.code)
    }
}

impl std::error::Error for InternalError {
    fn description(&self) -> &str {
        &self.msg
    }
}

#[derive(Debug)]
pub enum GitFSError {
    IOError(std::io::Error),
    ReqwestError(reqwest::Error),
    InternalError(InternalError),
    NoneError(std::option::NoneError),
}

impl GitFSError {
    pub fn new(msg: &str, code: i32) -> GitFSError {
        GitFSError::InternalError(InternalError{
            code: code,
            msg: msg.to_string(),
        })
    }

    pub fn code(&self) -> i32 {
        match *self {
            GitFSError::IOError(ref e) => {
                return e.raw_os_error().unwrap_or(libc::EIO);
            },
            GitFSError::ReqwestError(ref e) => {
                let status = e.status();
                if status.is_none() {
                    return libc::EIO;
                }
                match e.status().unwrap() {
                    StatusCode::OK => 0,
                    StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => libc::EPERM,
                    StatusCode::NOT_FOUND => libc::ENOENT,
                    StatusCode::REQUEST_TIMEOUT => libc::ETIMEDOUT,
                    StatusCode::PRECONDITION_FAILED => libc::EINVAL,
                    StatusCode::PAYLOAD_TOO_LARGE => libc::EFBIG,
                    StatusCode::TOO_MANY_REQUESTS => libc::EDQUOT,
                    StatusCode::INTERNAL_SERVER_ERROR => libc::EIO,
                    StatusCode::NOT_IMPLEMENTED => libc::ENOSYS,
                    s => {
                        eprintln!("Found an unknown HTTP code: {}", s);
                        libc::EIO
                    }
                }
            },
            GitFSError::InternalError(ref e) => {
                e.code
            },
            GitFSError::NoneError(_) => {
                // TODO: There may be a better code for this case.
                libc::EIO
            },
        }
    }
}

impl fmt::Display for GitFSError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GitFSError::IOError(ref e) => e.fmt(f),
            GitFSError::ReqwestError(ref e) => e.fmt(f),
            GitFSError::InternalError(ref e) => e.fmt(f),
            GitFSError::NoneError(ref e) => {
                write!(f, "None found: {:?}", e)
            },
        }
    }
}

impl error::Error for GitFSError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            GitFSError::IOError(ref e) => Some(e),
            GitFSError::ReqwestError(ref e) => Some(e),
            GitFSError::InternalError(ref e) => Some(e),
            GitFSError::NoneError(_) => None,
        }
    }
}


// This will be automatically called by `?` if a `io::Error`
// needs to be converted into a `GitFSError`.
impl From<std::io::Error> for GitFSError {
    fn from(err: std::io::Error) -> GitFSError {
        GitFSError::IOError(err)
    }
}

impl From<reqwest::Error> for GitFSError {
    fn from(err: reqwest::Error) -> GitFSError {
        GitFSError::ReqwestError(err)
    }
}

impl From<std::option::NoneError> for GitFSError {
    fn from(err: std::option::NoneError) -> GitFSError {
        GitFSError::NoneError(err)
    }
}

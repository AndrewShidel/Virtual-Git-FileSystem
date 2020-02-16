use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;

#[macro_use]
extern crate log;

#[macro_use]
extern crate rouille;

mod libc_extras;
mod libc_wrappers;
mod filesystem;
mod git;
mod github;
mod oauth;
//mod oauth2;

struct ConsoleLogger;

impl log::Log for ConsoleLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        println!("{}: {}: {}", record.target(), record.level(), record.args());
    }

    fn flush(&self) {}
}

static LOGGER: ConsoleLogger = ConsoleLogger;

fn main() {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Debug);
    let token = oauth::get_token().unwrap();
    let args: Vec<OsString> = env::args_os().collect();

    if args.len() != 3 {
        eprintln!("usage: {} <target> <mountpoint>", &env::args().next().unwrap());
        ::std::process::exit(1);
    }
    let github_repo = format!("{}/repos/github.com", args[1].to_str().unwrap());
    if let Err(e) = fs::create_dir_all(github_repo) {
        eprintln!("unable to create cache directory: {}", e);
        ::std::process::exit(1);
    }

    let filesystem = filesystem::PassthroughFS::new(args[1].clone(), token);

    let fuse_args: Vec<&OsStr> = vec![&OsStr::new("-o"), &OsStr::new("auto_unmount")];

    fuse_mt::mount(fuse_mt::FuseMT::new(filesystem, 1), &args[2], &fuse_args).unwrap();
}

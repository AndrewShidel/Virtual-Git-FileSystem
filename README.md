GitFS presents git repositories as files/directories by running a custom FUSE filesystem. It is useful for browsing code and depending on other repositories.

## Installation

### Linux Dependencies

Run all of the apt commands in `apt_deps.sh`.

### MacOS Dependencies

Download and install the latest version of [FUSE for macOS](https://osxfuse.github.io/).

### Common

Run `cargo run <cache_dir> <mountpoint>` using the rust nightly release.

Where `cache_dir` is the directory to store cached files and directories and `mountpoint` is the directory to mount the filesystem. `cache_dir` will be created for you if needed, but `mountpoint` must be created ahead of time.

The first time this is run the Oauth flow is initiated and a browser tab will be opened so that you can authorize. This is needed to access private repositories and to grant higher API request limits.

If you have already run GitFS, you may have to run `umount <mountpoint>` before running again.

## Usage

GitFS can be used like any other directory. It has the following structure:

```
<mountpoint>
└── github.com
    └── <username>
        └── <repo>
            ├── <directory>
            │   └── <file>
            └── <file>
```

When you access the filesystem for the first time the `github.com` directory will be empty. This is because there are too many Github users to display here. Simply run `cd <username>` (or access the directory in any other way) to create a directory for a user. 

The `github.com/<username>` directories will automatically be propagated with all of the repositories that the user has access to. The `github.com/<username>/<repo>` directories will contain the contents of the repository.

Note that the first time you access a file or directory it needs to be fetched from Github which will take a second. However, all further accesses to that file or directory will be much faster.


## Improvements in progress

This project is currently in the very early stages. There are still many known bugs, performance improvements, and missing features.

* Use the Github GraphQL API to improve performance.
* Support for non-Github repositories.
* Ability to create multiple clients.
  * Ability to sync clients to various points in history.
* Automount the filesystem.
* Windows support (via Docker/VM).
* Graceful stop/resume.
* Remove caches and state.

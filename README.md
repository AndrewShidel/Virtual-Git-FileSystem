VG (**V**irtual **G**it filesystem) presents git repositories as files/directories by running a custom FUSE filesystem. It is useful for browsing code and depending on other repositories.

## Installation

These instructions assume that you have rust installed and are running on Debian, Ubuntu, or a similar operating system. VG should work for all/most versions of Linux, however they have not yet been tested.

1. Run all of the apt commands in `apt_deps.sh`.
2. `cargo run <cache_dir> <mountpoint>`

Where `cache_dir` is the directory to store cached files and directories and `mountpoint` is the directory to mount the filesystem. `cache_dir` will be created for you if needed, but `mountpoint` must be created ahead of time.

The first time this is run the Oauth flow is initiated and a browser tab will be opened so that you can authorize. This is needed to access private repositories and to grant higher API request limits.

## Usage

VG can be used like any other directory. It has the following structure:

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
* Better caching of API requests.
* Support for non-Github repositories.
* Ability to create multiple clients.
  * Ability to sync clients to various points in history.
* Fetch the contents of the `.git` directory when requested.
* Automount the filesystem.
* Support for MacOS.

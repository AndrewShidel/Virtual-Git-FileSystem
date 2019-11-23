VG (**V**irtual **G**it filesystem) presents git repositories as files/directories by running a custom FUSE filesystem.

This project is currently in the very early stages and accesses git repositories naively by simply cloning the repo into a local cache directory when needed. The next step with this project is to download files only as needed. This can be done using a sparse clone, however this still requires downloading the git history which is the slowest part of the clone process. One compromise is to use "svn export ..." to download individual files (only works for github). However, when any git commands are run (can be detected by accesses to .git), the repo must be cloned properly. This will also involve using the github API to get metadata about files in the repo.

Installation:
1. Run all of the apt commands in apt_deps.sh.
2. cargo run / <mountpoint>

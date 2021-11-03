# grit

Progress

- [x] Introduction
- [x] Getting to know `.git`
- [x] The first commit
- [x] Making history
- [x] Growing trees
- [x] The index
- [x] Incremental change
- [x] First-class commands
- [x] Status report
- [x] The next commit
- [ ] The Myers diff algorithm

Features

- Stores blobs, commits, and directory trees in `git`-compatible format
- Uses index for detecting changes and creating commits
- Updates index incrementally in `grit add`
- Detects changes between workspace, index, and `HEAD` in `grit status`

![Screenshot of `grit status` vs. `git status` output](/resources/status.png)

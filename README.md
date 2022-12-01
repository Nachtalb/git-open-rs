# `git-open` in rust

A `git-open` command similar to paulirish/git-open written in rust.

## Usage

```
Usage: git-open [OPTIONS] [REMOTE]

Arguments:
  [REMOTE]  Which remote to open

Options:
  -c, --commit <COMMIT>  Commit hash
  -n, --no-branch        Don't open current branch
  -p, --path <PATH>      Path of the git repository
  -v, --verbose...       More output per occurrence
  -q, --quiet...         Less output per occurrence
  -h, --help             Print help information
```

## TODO

- [x] open current repository `git open` => `https://github.com/Nachtalb/git-open-rs/tree/master`
- [x] open given remote `git open my_remote` => `https://github.com/my_remote/git-open-rs/tree/master`
- [x] open specific hash `git open -c 1234abcd` => `https://github.com/my_remote/git-open-rs/tree/1234abcd`>
- [x] open repository at specific path `git open -p /my/git/repo` => `http://some.git/repo.git`
- [x] prevent opening of current branch `git open -n` => with branch `foobar`
      checked out `https://github.com/Nachtalb/git-open-rs/` instead of `https://github.com/Nachtalb/git-open-rs/tree/foobar`
- [x] Support resolving hosts through custom ssh configs
- [ ] open branch in correct remote (in case it doesn't exist in all remotes)

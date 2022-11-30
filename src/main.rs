use std::{env::current_dir, process::exit, io::BufReader, path::{Path, PathBuf}, ffi::OsStr};
use ssh2_config::SshConfig;
use url::Url;
use std::fs::File;
use git_url_parse::normalize_url;
use clap::Parser;
use path_absolutize::{self, Absolutize};

use git2::{Repository, Remote, Error};
use log::{info, warn, error, debug};

#[derive(Parser, Debug)]
struct Args {
    #[arg(required = false, help = "Which remote to open")]
    remote: Option<String>,

    #[arg(short, long, required = false, help = "Commit hash")]
    commit: Option<String>,

    #[arg(short, long, action, help = "Don't open current branch")]
    no_branch: bool,

    #[arg(short, long, required = false, help = "Path of the git repository")]
    path: Option<PathBuf>,

    #[arg(short, long, required = false, action, help = "Suppress output")]
    quiet: bool,
}

fn absolutize_and_expand<P: AsRef<OsStr>>(path: P) -> Result<PathBuf, String>{
    let path = match shellexpand::full(path.as_ref().to_str().unwrap()) {
        Ok(path) => path.into_owned(),
        Err(err) => return Err(format!("{}", err)),
    };
    Ok(Path::new(&path).absolutize().unwrap().into_owned())
}

fn main() {
    let arguments = Args::parse();

    stderrlog::new().module(module_path!()).quiet(arguments.quiet).verbosity(2).init().unwrap();

    let git_path = match arguments.path {
        Some(path) => {
            let path: PathBuf = match absolutize_and_expand(path) {
                Ok(path) => path,
                Err(err) => {
                    error!("Could not expand given path: {}", err);
                    exit(1)
                }
            };

            if path.exists() {
                path
            } else {
                error!("Path does not exist {}", path.display());
                exit(1);
            }
        },
        None => {
            match current_dir() {
                Ok(repo) => repo.absolutize().unwrap().into_owned(),
                Err(_) => {
                    error!("Could not get working directory.");
                    exit(1)
                }
            }
        }
    };

    let repo = match Repository::open(&git_path) {
        Ok(repo) => repo,
        Err(_) => {
            error!("{} is not a git repository", git_path.display());
            exit(1)
        }
    };

    let remote = match arguments.remote {
        Some(name) => match repo.find_remote(&name) {
            Ok(remote) => remote,
            Err(_) => {
                error!("Could not find remote named: {}", name);
                exit(1)
            },
        },
        None => match repo.find_remote("origin") {
            Ok(remote) => remote,
            Err(_) => {
                let remotes = repo.remotes().unwrap();
                if remotes.is_empty() {
                    error!("No remotes defined in repository");
                    exit(1)
                }
                repo.find_remote(remotes.get(0).unwrap()).unwrap()
            }
        }
    };

    match remote_to_url(&remote) {
        Ok(remote_url) => {
            let remote_url = connect_url_segments(remote_url, &repo, arguments.no_branch, arguments.commit);
            match webbrowser::open(&remote_url) {
                Ok(_) => info!("Opening remote {} [{}] => {remote_url}", remote.name().unwrap(), remote.url().unwrap()),
                Err(_) => error!("Could not open webbrowser. Here is the URL: {}", remote_url)
            };
        },
        Err(err) => {
            error!("{}: {}", err, remote.url().unwrap().to_string());
            exit(1)
        }
    };
}

fn get_checked_out_branch(repo: &Repository) -> Result<String, Error> {
    let branches = repo.branches(None)?;
    for branch in branches {
        match branch {
            Ok(b) => {
                if b.0.is_head() {
                    return Ok(String::from(b.0.name()?.unwrap()))
                }
            },
            Err(_) => {}
        }
    };
    Err(Error::from_str("No branch checked out"))
}

fn connect_url_segments(mut base: Url, repository: &Repository, no_branch: bool, hash: Option<String>) -> String {
    // Get the current commit hash if HEAD is given otherwise search for use the hash we received
    let commit_hash = match hash {
        Some(hash) if hash == "HEAD" => {
            match repository.head().unwrap().peel_to_commit() {
                Ok(commit) => Some(commit.id().to_string()),
                Err(_) => None,
            }
        },
        Some(hash) => Some(hash),
        None => None,
    };
    // Get current branch name, if needed it's remote companion else we let it be
    let branch = if !no_branch {
        match get_checked_out_branch(repository) {
            Ok(branch) => Some(match repository.branch_remote_name(branch.as_str()) {
                Ok(name) => name.as_str().unwrap_or(&branch).to_string(),
                Err(_) => branch.to_string(),
            }),
            Err(_) => None,
        }
    } else {
        None
    };

    let tree = if commit_hash.is_some() { commit_hash } else {branch};

    if let Some(segment) = tree {
        base.set_path(format!("{}/tree/{}", base.path(), segment).as_str())
    }

    base.to_string()
}

fn resolve_ssh_host(host: String) -> String {
    let config_path = match absolutize_and_expand("~/.ssh/config") {
        Ok(path) if !path.exists() => {
            info!("{}", path.display());
            match absolutize_and_expand("/etc/ssh/ssh_config") {
                Ok(path) if !path.exists() => return host,
                Ok(path) => path,
                Err(_) => return host
            }
        },
        Ok(path) => path,
        Err(_) => return host
    };


    let mut reader = match File::open(&config_path) {
        Ok(file) => BufReader::new(file),
        Err(err) => {
            warn!("Could not read ssh config file at {}, host might not be resolved correctly: {}", config_path.display(), err);
            return host
        }
    };

    let config = match SshConfig::default().parse(&mut reader) {
        Ok(config) => config,
        Err(err) => {
            warn!("Could not parse config file at {}, host might not be resolved correctly: {}", config_path.display(), err);
            return host
        },
    };

    match config.query(&host).host_name {
        Some(custom_host) => {
            debug!("SSH hostname resolved through ssh config {} from {} to {}", config_path.display(), host, custom_host);
            custom_host.to_string()
        },
        None => host,
    }
}

fn remote_to_url(remote: &Remote) -> Result<Url, String> {
    let format_error = Err(String::from("Don't know remote format"));
    let remote_url = match remote.url() {
        Some(url) if Path::new(url).exists() => return Err(String::from("Remote is a local path")),
        None => return format_error,
        Some(url) => normalize_url(url),
    };

    match remote_url {
        Ok(url) if ["http", "https"].contains(&url.scheme()) => {
            Ok(url)
        },
        Ok(url) if ["ssh", "git", ""].contains(&url.scheme()) && url.has_host() => {
            let mut host = url.host().unwrap().to_string();
            if url.scheme() == "ssh" {
                host = resolve_ssh_host(host);
            }

            let mut credentials = String::new();
            if url.password().is_some() && url.has_authority()  {
                credentials = format!("{}:{}@", url.username(), url.password().unwrap());
            }

            let mut path = url.path();
            if path.ends_with(".git") {
                path = &path[..path.len() - 4]
            }
            Ok(Url::parse(format!("https://{}{}/{}", credentials, host, path).as_str()).unwrap())
        },
        Ok(_) => return Err(String::from("Protocol not supported")),
        Err(_) => format_error,
    }
}


use std::{env::current_dir, process::exit, io::BufReader, path::{Path, PathBuf}, ffi::OsStr};
use ssh2_config::SshConfig;
use std::fs::File;
use git_url_parse::normalize_url;
use clap::Parser;
use path_absolutize::{self, Absolutize};

use git2::{Repository, Remote};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, required = false, help = "Commit hash")]
    commit: Option<String>,

    #[arg(short, long, action, help = "Don't open current branch")]
    no_branch: bool,

    #[arg(short, long, required = false, help = "Path of the git repository")]
    path: Option<PathBuf>,
}

fn absolutize_and_expand<P: AsRef<OsStr>>(path: P) -> Result<PathBuf, String>{
    let path = match shellexpand::env(path.as_ref().to_str().unwrap()) {
        Ok(path) => path.into_owned(),
        Err(err) => return Err(format!("{}", err)),
    };
    Ok(Path::new(&path).absolutize().unwrap().into_owned())
}

fn main() {
    let arguments = Args::parse();
    let git_path = match arguments.path {
        Some(path) => {
            let path: PathBuf = match absolutize_and_expand(path) {
                Ok(path) => path,
                Err(err) => {
                    println!("Could not expand given path: {}", err);
                    exit(1)
                }
            };

            if path.exists() {
                path
            } else {
                println!("Path does not exist {}", path.display());
                exit(1);
            }
        },
        None => {
            match current_dir() {
                Ok(repo) => repo.absolutize().unwrap().into_owned(),
                Err(_) => {
                    println!("Could not get working directory.");
                    exit(1)
                }
            }
        }
    };

    let repo = match Repository::open(&git_path) {
        Ok(repo) => repo,
        Err(_) => {
            println!("Could not get git information.");
            exit(1)
        }
    };

    let remote = match repo.remotes() {
        Ok(remotes) if !remotes.is_empty() => repo.find_remote(remotes.get(0).unwrap()).unwrap(),
        Ok(_) => {
            println!("No remotes defined in repository");
            exit(1)
        }
        Err(err) => {
            println!("Could not read remote information: {err}");
            exit(1)
        },
    };


    match remote_to_url(&remote) {
        Ok(remote_url) => {
            match webbrowser::open(&remote_url) {
                Ok(_) => println!("Opening url {remote_url}"),
                Err(_) => println!("Could not open webbrowser. Here is the URL: {}", remote_url)
            };
        },
        Err(err) => {
            println!("{}: {}", err, remote.url().unwrap().to_string());
            exit(1)
        }
    };
}

fn resolve_ssh_host(host: String) -> String {
    let config_path = match absolutize_and_expand("~/.ssh/config") {
        Ok(path) if !path.exists() => {
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
            println!("Could not read ssh config file at {}: {}", config_path.display(), err);
            return host
        }
    };

    let config = match SshConfig::default().parse(&mut reader) {
        Ok(config) => config,
        Err(err) => {
            println!("Could not parse config file at {}: {}", config_path.display(), err);
            return host
        },
    };

    match config.query(&host).host_name {
        Some(custom_host) => custom_host.to_string(),
        None => host,
    }
}

fn remote_to_url(remote: &Remote) -> Result<String, String> {
    let format_error = Err(String::from("Don't know remote format"));
    let remote_url = match remote.url() {
        Some(url) if Path::new(url).exists() => return Err(String::from("Remote is a local path")),
        None => return format_error,
        Some(url) => normalize_url(url),
    };

    match remote_url {
        Ok(url) if ["http", "https"].contains(&url.scheme()) => {
            Ok(String::from(url))
        },
        Ok(url) if ["ssh", "git", ""].contains(&url.scheme()) && url.has_host() => {
            let mut host = url.host().unwrap().to_string();
            if url.scheme() == "ssh" {
                host = resolve_ssh_host(host);
            }

            let mut credentials = String::new();
            if url.has_authority() {
               credentials = format!("{}:{}@", url.username(), url.password().unwrap());
            }
            Ok(String::from(format!("https://{}{}", credentials, host)))
        },
        Ok(_) => return Err(String::from("Protocol not supported")),
        Err(_) => format_error,
    }
}

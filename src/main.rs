use std::{env::current_dir, process::exit, io::BufReader, path::Path};
use url::Url;
use ssh2_config::SshConfig;
use std::fs::File;

use git2::{Repository, Remote};

fn main() {
    let pwd = match current_dir() {
        Ok(repo) => repo,
        Err(_) => {
            println!("Could not get working directory.");
            exit(1)
        }
    };

    let repo = match Repository::open(pwd.as_path()) {
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
            if webbrowser::open(&remote_url).is_err() {
                println!("Could not open webbrowser. Here is the URL: {}", remote_url);
            }
        },
        Err(err) => {
            println!("{}: {}", err, remote.url().unwrap().to_string());
            exit(1)
        }
    };
}

fn resolve_ssh_host(host: String) -> String {
    let config_path = Path::new("~/.ssh/config");
    if !config_path.exists() {
        let config_path = Path::new("/etc/ssh/ssh_config");
        if !config_path.exists() {
            return host
        }
    }

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
        Some(url) if !url.contains("://") => format!("ssh://{url}"),
        Some(url) => url.to_string(),
        None => return format_error
    };

    println!("{:#?}, {:#?}", remote_url, Url::parse(&remote_url));

    match Url::parse(&remote_url) {
        Ok(url) if ["http", "https"].contains(&url.scheme()) => {
            Ok(remote_url)
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

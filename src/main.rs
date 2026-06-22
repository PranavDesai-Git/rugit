#![cfg_attr(target_os = "windows", feature(windows_by_handle))]
pub mod base;
pub mod cli;
pub mod network;
pub mod pack;

fn main() {
    env_logger::init();

    match cli::parse_args() {
        Ok(command) => match command {
            cli::Command::Init => {
                base::init();
            }
            cli::Command::Add { filepath } => {
                if let Err(e) = base::stage_file(&filepath) {
                    eprintln!("Fatal error staging file: {}", e);
                    std::process::exit(1);
                }
            }
            cli::Command::Commit { message } => {
                let author = "host0";
                let email = "host0@host0";

                if let Err(e) = base::commit(&message, author, email) {
                    eprintln!("Fatal error during commit: {}", e);
                    std::process::exit(1);
                }
            }
            cli::Command::RemoteAdd { name, url } => {
                if let Err(e) = base::add_remote(&name, &url) {
                    eprintln!("Fatal error configuring remote: {}", e);
                    std::process::exit(1);
                }
            }
            cli::Command::Push { remote_name_or_url } => {
                // Determine our target url based on optional input
                let target_url = match remote_name_or_url {
                    Some(val) => {
                        if val.starts_with("http://") || val.starts_with("https://") {
                            val
                        } else {
                            match base::get_remote_url(&val) {
                                Ok(url) => url,
                                Err(e) => {
                                    eprintln!("Fatal error resolving remote name '{}': {}", val, e);
                                    std::process::exit(1);
                                }
                            }
                        }
                    }
                    None => {
                        match base::get_remote_url("origin") {
                            Ok(url) => url,
                            Err(_) => {
                                eprintln!("Error: No remote specified, and default 'origin' is not configured.");
                                eprintln!("Run: rugit remote add origin <url>");
                                std::process::exit(1);
                            }
                        }
                    }
                };

                network::test_push_connection(&target_url);
            }
            cli::Command::Help => {
                cli::print_help();
            }
        },
        Err(err_msg) => {
            eprintln!("{}", err_msg);
            std::process::exit(1);
        }
    }
}

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
                let (author, email) = base::get_user_identity();

                if email == "unknown@example.com" {
                    println!(
                        "Warning: Commit email is unset. GitHub won't recognize this account."
                    );
                    println!("Hint: Run 'rugit config user.email <email>' to fix this.");
                }

                if let Err(e) = base::commit(&message, &author, &email) {
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
            cli::Command::ConfigSet { key, value } => {
                if let Err(e) = base::set_user_config(&key, &value) {
                    eprintln!("Fatal error updating configuration: {}", e);
                    std::process::exit(1);
                }
                println!("Successfully configured user.{}!", key);
            }
            cli::Command::BranchCreate { name } => {
                if let Err(e) = base::create_branch(&name) {
                    eprintln!("Fatal error creating branch: {}", e);
                    std::process::exit(1);
                }
            }
            cli::Command::Switch { name } => {
                if let Err(e) = base::switch_branch(&name) {
                    eprintln!("Fatal error switching branch: {}", e);
                    std::process::exit(1);
                }
            }
            cli::Command::Push { remote_name_or_url } => {
                let active_branch_ref = match base::get_current_branch_ref() {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Fatal: Could not determine active branch from HEAD: {}", e);
                        std::process::exit(1);
                    }
                };

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
                    None => match base::get_remote_url("origin") {
                        Ok(url) => url,
                        Err(_) => {
                            eprintln!(
                                "Error: No remote specified, and default 'origin' is not configured."
                            );
                            eprintln!("Run: rugit remote add origin <url>");
                            std::process::exit(1);
                        }
                    },
                };

                network::push_remote(&target_url, &active_branch_ref);
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

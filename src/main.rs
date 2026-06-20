#![feature(windows_by_handle)]
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
            cli::Command::Push { remote_url } => {
                network::test_push_connection(&remote_url);
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

use std::env;

#[derive(Debug)]
pub enum Command {
    Init,
    Add { filepath: String },
    Commit { message: String },
    RemoteAdd { name: String, url: String },
    Push { remote_name_or_url: Option<String> }, 
    Help,
}

pub fn parse_args() -> Result<Command, String> {
    let mut args = env::args().skip(1); // Skip the binary execution path

    let sub_command = match args.next() {
        Some(cmd) => cmd,
        None => return Ok(Command::Help),
    };

    match sub_command.as_str() {
        "init" => Ok(Command::Init),
        "add" => {
            let filepath = args
                .next()
                .ok_or("Error: 'add' requires a file path. Usage: rugit add <file>")?;
            Ok(Command::Add { filepath })
        }
        "commit" => {
            let mut message = String::new();

            while let Some(arg) = args.next() {
                if arg == "-m" || arg == "--message" {
                    message = args
                        .next()
                        .ok_or("Error: -m requires a commit message string.")?;
                    break;
                }
            }

            if message.is_empty() {
                return Err(
                    "Error: rugit requires a commit message. Use -m \"your message\"".to_string(),
                );
            }

            Ok(Command::Commit { message })
        }
        "remote" => {
            let sub_action = args
                .next()
                .ok_or("Error: 'remote' requires an action. Usage: rugit remote add <name> <url>")?;

            if sub_action != "add" {
                return Err(format!(
                    "Error: Unknown remote action '{}'. Did you mean 'add'?",
                    sub_action
                ));
            }

            let name = args
                .next()
                .ok_or("Error: 'remote add' requires a name. Usage: rugit remote add <name> <url>")?;
            
            let url = args
                .next()
                .ok_or("Error: 'remote add' requires a URL. Usage: rugit remote add <name> <url>")?;

            Ok(Command::RemoteAdd { name, url })
        }
        "push" => {
            let remote_name_or_url = args.next();
            Ok(Command::Push { remote_name_or_url })
        }
        "-h" | "--help" | "help" => Ok(Command::Help),
        unknown => Err(format!(
            "Error: Unknown command '{}'. Run 'rugit help' for usage.",
            unknown
        )),
    }
}

pub fn print_help() {
    println!(
        r#"
rugit - A minimalist Git core implementation in Rust

USAGE:
    rugit <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    init                      Initialize a new local plumbing repository (.git/)
    add <file>                Stage a file to the index area
    commit -m "<msg>"         Commit staged changes to the object history
    remote add <name> <url>   Track a new remote destination repository
    push [destination]        Negotiate with remote host and push upstream (defaults to origin)
    help, -h, --help          Print this help information
"#
    );
}

use std::env;

#[derive(Debug)]
pub enum Command {
    Init,
    Add { filepath: String },
    Commit { message: String },
    RemoteAdd { name: String, url: String },
    Push { remote_name_or_url: Option<String> },
    ConfigSet { key: String, value: String },
    Help,
}

pub fn parse_args() -> Result<Command, String> {
    let mut args = env::args().skip(1);

    let sub_command = match args.next() {
        Some(cmd) => cmd,
        None => return Ok(Command::Help),
    };

    match sub_command.as_str() {
        "init" => Ok(Command::Init),
        "add" => {
            let filepath = args.next().ok_or("Error: 'add' requires a file path.")?;
            Ok(Command::Add { filepath })
        }
        "commit" => {
            let mut message = String::new();
            while let Some(arg) = args.next() {
                if arg == "-m" || arg == "--message" {
                    message = args.next().ok_or("Error: -m requires a commit message.")?;
                    break;
                }
            }
            if message.is_empty() {
                return Err("Error: rugit requires a commit message.".to_string());
            }
            Ok(Command::Commit { message })
        }
        "remote" => {
            let sub_action = args
                .next()
                .ok_or("Error: 'remote' requires an action like 'add'.")?;
            if sub_action != "add" {
                return Err(format!("Error: Unknown remote action '{}'.", sub_action));
            }
            let name = args.next().ok_or("Error: Missing remote name.")?;
            let url = args.next().ok_or("Error: Missing remote URL.")?;
            Ok(Command::RemoteAdd { name, url })
        }
        "config" => {
            let full_key = args
                .next()
                .ok_or("Error: 'config' requires a key. Usage: rugit config user.name \"Value\"")?;
            let value = args.next().ok_or(
                "Error: 'config' requires a value. Usage: rugit config user.name \"Value\"",
            )?;

            let prop = full_key
                .strip_prefix("user.")
                .ok_or("Error: Only 'user.name' and 'user.email' keys are currently supported.")?;

            if prop != "name" && prop != "email" {
                return Err(
                    "Error: Invalid property configuration. Supported: user.name, user.email"
                        .to_string(),
                );
            }

            Ok(Command::ConfigSet {
                key: prop.to_string(),
                value,
            })
        }
        "push" => {
            let remote_name_or_url = args.next();
            Ok(Command::Push { remote_name_or_url })
        }
        "-h" | "--help" | "help" => Ok(Command::Help),
        unknown => Err(format!("Error: Unknown command '{}'.", unknown)),
    }
}

pub fn print_help() {
    println!(
        r#"
rugit - A minimalist Git core implementation in Rust

USAGE:
    rugit <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    init                        Initialize a new local plumbing repository (.git/)
    add <file>                  Stage a file to the index area
    commit -m "<msg>"           Commit staged changes to the object history
    config <key> <value>        Set configuration settings (e.g., user.name, user.email)
    remote add <name> <url>     Track a new remote destination repository
    push [destination]          Negotiate with remote host and push upstream (defaults to origin)
    help, -h, --help            Print this help information
"#
    );
}

use std::env;

#[derive(Debug)]
pub enum Command {
    Init,
    Add { filepath: String },
    Commit { message: String },
    Push,
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
        "push" => Ok(Command::Push),
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
    init              Initialize a new local plumbing repository (.git/)
    add <file>        Stage a file to the index area
    commit -m "<msg>" Commit staged changes to the object history
    push              Negotiate with a remote host and push upstream
    help, -h, --help  Print this help information
"#
    );
}

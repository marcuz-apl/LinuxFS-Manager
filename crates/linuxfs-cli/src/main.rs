use std::{env, io};

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let result = match linuxfs_cli::parse_args(&args) {
        Ok(command) => linuxfs_cli::run(command, &mut io::stdout()),
        Err(message) => {
            eprintln!("{message}");
            return;
        }
    };
    if let Err(error) = result {
        eprintln!("linuxfs: {error}");
        std::process::exit(1);
    }
}

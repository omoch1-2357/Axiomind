use std::io::{self};

fn main() {
    let code = axiomind_cli::run(std::env::args(), &mut io::stdout(), &mut io::stderr());
    std::process::exit(code);
}

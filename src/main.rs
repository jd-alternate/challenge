use std::{env, error::Error, fs::File, io};

// This program takes a command-line argument that points to
// an input CSV file of events, reads the events from it, and writes the
// resulting state to an output CSV file.

fn main() -> Result<(), Box<dyn Error>> {
    let file = get_file_from_cli_arg()?;
    let mut input = io::BufReader::new(file);

    // `run` takes a writer for logging errors but we're skipping that
    // here because it wasn't in the spec and the faster, the better. We could
    // easily swap out io::sink for io::stderr
    challenge::run(&mut input, &mut io::stdout(), &mut io::sink())
}

fn get_file_from_cli_arg() -> Result<File, Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err(format!("Usage: {} <filename>", args[0]).into());
    }

    let path = &args[1];
    let file = File::open(&path)?;
    Ok(file)
}

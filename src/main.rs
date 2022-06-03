// TODO: use clippy and auto-merge
use std::env;
use std::error::Error;
use std::fs::File;
use std::io;
// TODO: think of better name
mod client;
mod parsing;
mod processing;
mod types;

fn main() -> Result<(), Box<dyn Error>> {
    let mut file = get_file()?;

    // TODO: consider buffered reader
    let events_iter = parsing::csv::csv_iterator(&mut file);
    let result = processing::process_events(events_iter)?;
    parsing::csv::write_result(result, io::stdout())?;

    Ok(())
}

fn get_file() -> Result<File, Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        // TODO: return actual error
        eprintln!("Expected exactly one argument containing the path to a CSV input file");
        std::process::exit(1);
    }
    let path = &args[1];
    let file = File::open(&path)?;
    Ok(file)
}

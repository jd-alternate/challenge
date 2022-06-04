// TODO: use clippy and auto-merge
use std::{
    env,
    error::Error,
    fs::File,
    io::{self, Read, Write},
};
mod client;
mod parsing;
mod processing;
mod types;

fn main() -> Result<(), Box<dyn Error>> {
    let file = get_file()?;
    let mut input = io::BufReader::new(file);

    run(&mut input, &mut io::stdout())
}

fn run(input: &mut impl Read, output: &mut impl Write) -> Result<(), Box<dyn Error>> {
    let events_iter = parsing::csv::to_events_iter(input);

    // we're logging errors for the sake of easier testing and debugging, but
    // we're not logging here just because it wasn't in the spec.
    // io::sink could easily be swapped out for io::stderr.
    let final_state = processing::process_events(events_iter, &mut io::sink())?;

    parsing::csv::write_result(final_state, output)?;

    Ok(())
}

fn get_file() -> Result<File, Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err(format!("Usage: {} <filename>", args[0]).into());
    }

    let path = &args[1];
    let file = File::open(&path)?;
    Ok(file)
}

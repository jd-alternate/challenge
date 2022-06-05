use std::{
    env,
    error::Error,
    fs::File,
    io::{self, Read, Write},
};
mod format;
mod model;
mod system;

// From a high-level, this library takes a command-line argument that points to
// an input CSV file of events, reads the events from it, and writes the
// resulting state to an output CSV file.

pub fn run() -> Result<(), Box<dyn Error>> {
    let file = get_file_from_cli_arg()?;
    let mut input = io::BufReader::new(file);

    run_aux(&mut input, &mut io::stdout())
}

// This is a more generic version of `run` which simply takes an input and
// output, for ease of testing.
#[inline]
pub fn run_aux(input: &mut impl Read, output: &mut impl Write) -> Result<(), Box<dyn Error>> {
    let events_iter = format::csv::input::parse_events(input);

    // `process_events` takes a writer for logging errors but we're skipping that
    // here because it wasn't in the spec and the faster, the better. We could
    // easily swap out io::sink for io::stderr
    let final_state = system::process_events(events_iter, &mut io::sink())?;

    format::csv::output::write_report(final_state, output)?;

    Ok(())
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

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_run_aux() {
        let input = concat!(
            "type,client,tx,    amount\n",
            "deposit,1, 1, 1.11111\n",
            "deposit,2,2,2.0\n",
            "deposit,1,3,   2.0\n",
            "withdrawal,1,4     ,1.5   \n",
            "withdrawal,2,5,3.0\n",
        );
        let expected_output = concat!(
            "client,available,held,total,locked\n",
            "1,1.61111,0,1.61111,false\n",
            "2,2.0,0,2.0,false\n"
        );

        let mut output = Vec::new();
        run_aux(&mut input.as_bytes(), &mut output).expect("Unexpected error");

        let output_str = String::from_utf8(output).expect("Not UTF-8");

        assert_eq!(expected_output, output_str);
    }
}

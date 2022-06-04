use std::{
    env,
    error::Error,
    fs::File,
    io::{self, Read, Write},
};
mod client;
mod format;
mod processing;
mod types;

pub fn run() -> Result<(), Box<dyn Error>> {
    let file = get_file()?;
    let mut input = io::BufReader::new(file);

    run_aux(&mut input, &mut io::stdout())
}

fn run_aux(input: &mut impl Read, output: &mut impl Write) -> Result<(), Box<dyn Error>> {
    let events_iter = format::csv::to_events_iter(input);

    // we're logging errors for the sake of easier testing and debugging, but
    // we're not logging here just because it wasn't in the spec.
    // io::sink could easily be swapped out for io::stderr.
    let final_state = processing::process_events(events_iter, &mut io::sink())?;

    format::csv::write_result(final_state, output)?;

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

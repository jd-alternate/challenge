use std::{
    error::Error,
    io::{Read, Write},
};
mod format;
mod model;
mod system;

#[inline]
pub fn run(
    input: &mut impl Read,
    output: &mut impl Write,
    err_output: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let events_iter = format::csv::input::parse_events(input);

    let final_state = system::process_events(events_iter, err_output)?;

    format::csv::output::write_report(final_state, output)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use std::io;

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_run() {
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
        run(&mut input.as_bytes(), &mut output, &mut io::sink()).expect("Unexpected error");

        let output_str = String::from_utf8(output).expect("Not UTF-8");

        assert_eq!(expected_output, output_str);
    }
}

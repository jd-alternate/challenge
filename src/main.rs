// TODO: use clippy and auto-merge
use serde::Deserialize;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::io::{self, Read};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Expected exactly one argument containing the path to a CSV input file");
        std::process::exit(1);
    }
    let path = &args[1];
    let mut file = File::open(&path)?;

    // TODO: consider buffered reader
    let result = parse(csv_iterator(&mut file))?;
    write_result(result)?;

    Ok(())
}

fn write_result(result: Vec<Client>) -> Result<(), Box<dyn Error>> {
    // going to write to stdout
    let mut writer = io::stdout();
    writer.write(b"client,available,held,total,locked")?;
    for client in result {
        writer.write(client.to_csv_row().as_bytes())?;
    }

    Ok(())
}

// this will return an iterator which itself yields InputEvents
// TODO: genericise that error message because we're trying to encapsulate the CSV part
fn csv_iterator(reader: impl Read) -> impl Iterator<Item = Result<InputEvent, csv::Error>> {
    csv::Reader::from_reader(reader).into_deserialize()
}

// I really want to receive an iterator that gives me actual events. That will be much easier to test with.
fn parse(
    events: impl Iterator<Item = Result<InputEvent, csv::Error>>,
) -> Result<Vec<Client>, Box<dyn Error>> {
    // how do I actually process this stuff? Let's start with

    for result in events {
        let record: InputEvent = result?;
        println!("{:?}", record);
    }

    Ok(vec![])
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_empty() {
        // just going to pass an empty reader
        let input_events = vec![];
        let result = parse(input_events.into_iter()).expect("failed to parse");
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_single_event() {
        // just going to pass an empty reader
        let input_events = vec![Ok(InputEvent {
            kind: "deposit".to_string(),
            client_id: 1,
            transaction_id: 1,
            amount: Some(100),
        })];
        let result = parse(input_events.into_iter()).expect("failed to parse");
        assert_eq!(result, vec![]);
    }
}

// TODO: consider using decimal type
// TODO: handle 4 digits after decimal point
type Amount = u32;
type TransactionID = u32;
type ClientID = u16;

#[derive(Debug, Deserialize)]
enum TransactionType {
    Deposit(Amount),
    Withdrawal(Amount),
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Deserialize)]
struct Transaction {
    kind: TransactionType,
    id: TransactionID,
    client_id: ClientID,
}

#[derive(Debug, Deserialize)]
struct InputEvent {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "client")]
    client_id: ClientID,
    #[serde(rename = "tx")]
    transaction_id: TransactionID,
    amount: Option<Amount>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Client {
    id: ClientID,
    held: Amount,
    total: Amount,
    locked: bool,
}

impl Client {
    fn new(id: ClientID) -> Self {
        Self {
            id,
            held: 0,
            total: 0,
            locked: false,
        }
    }

    fn available(&self) -> Amount {
        self.total - self.held
    }

    // TODO: should this instead return direct bytes?
    fn to_csv_row(&self) -> String {
        format!(
            "{},{},{},{},{}",
            self.id,
            self.available(),
            self.held,
            self.total,
            self.locked
        )
    }
}

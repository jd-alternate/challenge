use core::str::FromStr;

use serde::Deserialize;
use std::{error::Error, io::Read};

use crate::model::{Amount, ClientID, DisputeStepKind, Event, TransactionID, TransactionKind};

#[derive(Deserialize)]
// intermediary struct for deserializing CSV
pub struct CsvEvent {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "tx")]
    transaction_id: TransactionID,
    #[serde(rename = "client")]
    client_id: ClientID,
    // We could use a custom deserializer that works with the rust decimal library's serde
    // deserializer, but it's pretty hairy to have that gracefully deal with empty strings, so
    // I'm just having serde treat this as a string and then I'm manually mapping to a decimal
    // afterwards.
    amount: String,
}

// Returns an iterator which itself yields Events. It takes a reader that
// reads a CSV file.
pub fn parse_events(reader: impl Read) -> impl Iterator<Item = Result<Event, Box<dyn Error>>> {
    csv::ReaderBuilder::new()
        .trim(csv::Trim::All) // this handles whitespace for us
        .from_reader(reader)
        .into_deserialize()
        .map(|result| parse_csv_event(result.map_err(|e| e.to_string())?))
}

fn parse_csv_event(csv_event: CsvEvent) -> Result<Event, Box<dyn Error>> {
    let event = match csv_event.kind.as_ref() {
        "deposit" => Event::Transaction {
            kind: TransactionKind::Deposit,
            transaction_id: csv_event.transaction_id,
            client_id: csv_event.client_id,
            amount: parse_amount(&csv_event.amount)?,
        },
        "withdrawal" => Event::Transaction {
            kind: TransactionKind::Withdrawal,
            transaction_id: csv_event.transaction_id,
            client_id: csv_event.client_id,
            amount: parse_amount(&csv_event.amount)?,
        },
        "dispute" => Event::DisputeStep {
            kind: DisputeStepKind::Dispute,
            transaction_id: csv_event.transaction_id,
            client_id: csv_event.client_id,
        },
        "resolve" => Event::DisputeStep {
            kind: DisputeStepKind::Resolve,
            transaction_id: csv_event.transaction_id,
            client_id: csv_event.client_id,
        },
        "chargeback" => Event::DisputeStep {
            kind: DisputeStepKind::Chargeback,
            transaction_id: csv_event.transaction_id,
            client_id: csv_event.client_id,
        },
        _ => return Err(format!("Unknown event kind: {}.", csv_event.kind).into()),
    };

    Ok(event)
}

fn parse_amount(amount: &str) -> Result<Amount, Box<dyn Error>> {
    if amount.is_empty() {
        return Err("Missing amount.".into());
    }

    Ok(Amount::from_str(amount)?)
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;
    use rust_decimal_macros::dec;

    #[test]
    fn test_parse_events_empty_file() {
        let input = String::new();
        let mut events_iter = parse_events(input.as_bytes());
        assert!(events_iter.next().is_none());
    }

    #[test]
    fn test_parse_events_all_event_types() {
        let input = concat!(
            "type,client,tx,    amount\n",
            "deposit,1,2,3.12345\n",
            "withdrawal,4,5,6\n",
            "dispute,7,8,\n",
            "resolve,9,10,\n",
            "chargeback,11,12,\n",
        );

        let events_iter = parse_events(input.as_bytes());
        let result = events_iter
            .collect::<Result<Vec<_>, _>>()
            .expect("Expected no errors.");

        assert_eq!(
            vec![
                Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id: 1,
                    transaction_id: 2,
                    amount: dec!(3.12345),
                },
                Event::Transaction {
                    kind: TransactionKind::Withdrawal,
                    client_id: 4,
                    transaction_id: 5,
                    amount: dec!(6),
                },
                Event::DisputeStep {
                    kind: DisputeStepKind::Dispute,
                    client_id: 7,
                    transaction_id: 8,
                },
                Event::DisputeStep {
                    kind: DisputeStepKind::Resolve,
                    client_id: 9,
                    transaction_id: 10,
                },
                Event::DisputeStep {
                    kind: DisputeStepKind::Chargeback,
                    client_id: 11,
                    transaction_id: 12,
                }
            ],
            result,
        );
    }

    #[test]
    fn test_parse_events_malformed_row() {
        let input = concat!(
            "type,client,tx,    amount\n",
            "deposit,1,1,1\n",
            "invalid\n",
            "deposit,2,2,2\n",
        );
        let events_iter = parse_events(input.as_bytes());
        let result = events_iter.collect::<Vec<_>>();
        assert_eq!(3, result.len());

        match result.get(0) {
            Some(Ok(event)) => assert_eq!(
                Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id: 1,
                    transaction_id: 1,
                    amount: dec!(1),
                },
                *event,
            ),
            Some(Err(err)) => panic!("Unexpected error: {}", err),
            None => panic!("Expected Some"),
        }

        assert!(result.get(1).unwrap().is_err());

        match result.get(2) {
            Some(Ok(event)) => assert_eq!(
                Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id: 2,
                    transaction_id: 2,
                    amount: dec!(2),
                },
                *event,
            ),
            Some(Err(err)) => panic!("Unexpected error: {}", err),
            None => panic!("Expected Some"),
        }
    }

    #[test]
    fn test_parse_events_unknown_type() {
        let input = concat!("type,client,tx,    amount\n", "unknown,1,1,1\n",);
        let events_iter = parse_events(input.as_bytes());
        let result = events_iter.collect::<Vec<_>>();
        assert_eq!(1, result.len());

        match result.first() {
            Some(Err(err)) => assert_eq!("Unknown event kind: unknown.", err.to_string()),
            Some(Ok(_)) => panic!("Expected failed event parse"),
            None => panic!("Expected Some"),
        };
    }

    #[test]
    fn test_parse_events_missing_amount() {
        let input = concat!("type,client,tx,    amount\n", "deposit,1,1,\n",);
        let events_iter = parse_events(input.as_bytes());
        let result = events_iter.collect::<Vec<_>>();
        assert_eq!(1, result.len());

        match result.first() {
            Some(Err(err)) => assert_eq!("Missing amount.", err.to_string()),
            Some(Ok(_)) => panic!("Expected failed event parse"),
            None => panic!("Expected Some"),
        };
    }
}

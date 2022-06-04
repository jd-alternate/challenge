// Everything CSV-related lives here.

use core::str::FromStr;

use serde::Deserialize;
use std::{
    collections::HashMap,
    error::Error,
    io::{Read, Write},
};

use crate::model::{Amount, Client, ClientID, Event, TransactionID};

#[derive(Debug, Deserialize, PartialEq, Eq)]
// intermediary struct for deserializing CSV
pub struct CsvEvent {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "tx")]
    transaction_id: TransactionID,
    #[serde(rename = "client")]
    client_id: ClientID,
    // We could use a custom deserializer that works with the rust decimal library's serde deserializer,
    // but it's pretty hairy to have that gracefully deal with empty strings, so I'm just
    // having serde treat this as a string and then I'm manually mapping to a decimal afterwards.
    amount: String,
}

// Returns an iterator which itself yields Events. It takes a reader that
// wraps a CSV file.
pub fn to_events_iter(reader: impl Read) -> impl Iterator<Item = Result<Event, Box<dyn Error>>> {
    csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader)
        .into_deserialize()
        .map(|result| parse_csv_event(result.map_err(|e| e.to_string())?))
}

fn parse_csv_event(csv_event: CsvEvent) -> Result<Event, Box<dyn Error>> {
    match csv_event.kind.as_ref() {
        "deposit" => Ok(Event::Deposit {
            transaction_id: csv_event.transaction_id,
            client_id: csv_event.client_id,
            amount: parse_amount(&csv_event.amount)?,
        }),
        "withdrawal" => Ok(Event::Withdrawal {
            transaction_id: csv_event.transaction_id,
            client_id: csv_event.client_id,
            amount: parse_amount(&csv_event.amount)?,
        }),
        "dispute" => Ok(Event::Dispute {
            transaction_id: csv_event.transaction_id,
            client_id: csv_event.client_id,
        }),
        "resolve" => Ok(Event::Resolve {
            transaction_id: csv_event.transaction_id,
            client_id: csv_event.client_id,
        }),
        "chargeback" => Ok(Event::Chargeback {
            transaction_id: csv_event.transaction_id,
            client_id: csv_event.client_id,
        }),
        _ => Err(format!("Unknown event kind: {}.", csv_event.kind).into()),
    }
}

fn parse_amount(amount: &str) -> Result<Amount, Box<dyn Error>> {
    if amount.is_empty() {
        return Err("Missing amount.".into());
    }

    Ok(Amount::from_str(amount)?)
}

// Takes the resultant clients after processing events, and writes them to the
// given writer in CSV form.
pub fn write_result(
    final_state: HashMap<ClientID, Client>,
    mut writer: impl Write,
) -> Result<(), Box<dyn Error>> {
    writer.write_all(b"client,available,held,total,locked\n")?;
    let mut entries: Vec<(ClientID, Client)> = final_state.into_iter().collect();
    // This sorting is admittedly mostly for the sake of making testing easier,
    // though I assume that actually producing a report is a small part that happens
    // at the end of a long process of processing events, and I also assume that
    // it's convenient to order records by client ID despite the spec being indifferent.
    // If this assumption proves invalid we can ditch the sorting and just update the test.
    entries.sort_by(|(a, _), (b, _)| a.cmp(b));
    for (client_id, client) in entries {
        writer.write_all(to_csv_row(client_id, &client).as_bytes())?;
    }

    Ok(())
}

fn to_csv_row(client_id: ClientID, client: &Client) -> String {
    format!(
        "{},{},{},{},{}\n",
        client_id,
        client.get_available(),
        client.get_held(),
        client.get_total(),
        client.get_locked()
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;
    use rust_decimal_macros::dec;

    #[test]
    fn test_to_events_iter_empty_file() {
        let input = String::new();
        let mut events_iter = to_events_iter(input.as_bytes());
        assert!(events_iter.next().is_none());
    }

    #[test]
    fn test_to_events_iter_all_event_types() {
        let input = concat!(
            "type,client,tx,    amount\n",
            "deposit,1,2,3.12345\n",
            "withdrawal,4,5,6\n",
            "dispute,7,8,\n",
            "resolve,9,10,\n",
            "chargeback,11,12,\n",
        );

        let events_iter = to_events_iter(input.as_bytes());
        let result = events_iter
            .collect::<Result<Vec<_>, _>>()
            .expect("Expected no errors.");

        assert_eq!(
            vec![
                Event::Deposit {
                    client_id: 1,
                    transaction_id: 2,
                    amount: dec!(3.12345),
                },
                Event::Withdrawal {
                    client_id: 4,
                    transaction_id: 5,
                    amount: dec!(6),
                },
                Event::Dispute {
                    client_id: 7,
                    transaction_id: 8,
                },
                Event::Resolve {
                    client_id: 9,
                    transaction_id: 10,
                },
                Event::Chargeback {
                    client_id: 11,
                    transaction_id: 12,
                }
            ],
            result,
        );
    }

    #[test]
    fn test_to_events_iter_malformed_row() {
        let input = concat!(
            "type,client,tx,    amount\n",
            "deposit,1,1,1\n",
            "invalid\n",
            "deposit,2,2,2\n",
        );
        let events_iter = to_events_iter(input.as_bytes());
        let result = events_iter.collect::<Vec<_>>();
        assert_eq!(3, result.len());

        match result.get(0) {
            Some(Ok(event)) => assert_eq!(
                Event::Deposit {
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
                Event::Deposit {
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
    fn test_to_events_iter_unknown_type() {
        let input = concat!("type,client,tx,    amount\n", "unknown,1,1,1\n",);
        let events_iter = to_events_iter(input.as_bytes());
        let result = events_iter.collect::<Vec<_>>();
        assert_eq!(1, result.len());

        match result.first() {
            Some(Err(err)) => assert_eq!("Unknown event kind: unknown.", err.to_string()),
            Some(Ok(_)) => panic!("Expected failed event parse"),
            None => panic!("Expected Some"),
        };
    }

    #[test]
    fn test_to_events_iter_missing_amount() {
        let input = concat!("type,client,tx,    amount\n", "deposit,1,1,\n",);
        let events_iter = to_events_iter(input.as_bytes());
        let result = events_iter.collect::<Vec<_>>();
        assert_eq!(1, result.len());

        match result.first() {
            Some(Err(err)) => assert_eq!("Missing amount.", err.to_string()),
            Some(Ok(_)) => panic!("Expected failed event parse"),
            None => panic!("Expected Some"),
        };
    }

    #[test]
    fn test_write_results() {
        let mut writer = Vec::new();
        let result = HashMap::from([
            (1, Client::from(dec!(20), dec!(100), true)),
            (2, Client::from(dec!(6), dec!(7), false)),
        ]);

        write_result(result, &mut writer).expect("Expected no errors.");

        let output = String::from_utf8(writer).expect("Not UTF-8");
        assert_eq!(
            concat!(
                "client,available,held,total,locked\n",
                "1,80,20,100,true\n",
                "2,1,6,7,false\n"
            ),
            output,
        );
    }
}

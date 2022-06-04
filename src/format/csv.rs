// Everything CSV-related lives here.

use serde::Deserialize;
use std::{
    collections::HashMap,
    error::Error,
    io::{Read, Write},
};

use crate::{
    client::Client,
    processing::Event,
    types::{Amount, ClientID, TransactionID},
};

#[derive(Debug, Deserialize, PartialEq, Eq)]
// intermediary struct for deserializing CSV
pub struct CsvEvent {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "tx")]
    transaction_id: TransactionID,
    #[serde(rename = "client")]
    client_id: ClientID,
    amount: Option<Amount>,
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
            amount: csv_event.amount.ok_or("Missing amount.")?,
        }),
        "withdrawal" => Ok(Event::Withdrawal {
            transaction_id: csv_event.transaction_id,
            client_id: csv_event.client_id,
            amount: csv_event.amount.ok_or("Missing amount.")?,
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
        client.available(),
        client.held,
        client.total,
        client.locked
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_to_events_iter_empty_file() {
        let input = String::new();
        let events_iter =
            to_events_iter(input.as_bytes()).map(|result| result.map_err(|e| e.to_string()));
        assert_eq!(events_iter.collect::<Vec<_>>(), vec![]);
    }

    #[test]
    fn test_to_events_iter_all_event_types() {
        let input = String::from("type,client,tx,amount\ndeposit,1,2,3\nwithdrawal,4,5,6\ndispute,7,8,\nresolve,9,10,\nchargeback,11,12,\n");
        let events_iter = to_events_iter(input.as_bytes());
        let result = events_iter
            .collect::<Result<Vec<_>, _>>()
            .expect("Expected no errors.");

        assert_eq!(
            result,
            vec![
                Event::Deposit {
                    client_id: 1,
                    transaction_id: 2,
                    amount: 3,
                },
                Event::Withdrawal {
                    client_id: 4,
                    transaction_id: 5,
                    amount: 6,
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
            ]
        );
    }

    #[test]
    fn test_to_events_iter_malformed_row() {
        let input = String::from("type,client,tx,amount\ndeposit,1,1,1\ninvalid\ndeposit,2,2,2");
        let events_iter = to_events_iter(input.as_bytes());
        let result = events_iter.collect::<Vec<_>>();
        assert_eq!(result.len(), 3);

        match result.get(0) {
            Some(Ok(event)) => assert_eq!(
                *event,
                Event::Deposit {
                    client_id: 1,
                    transaction_id: 1,
                    amount: 1,
                }
            ),
            Some(Err(err)) => panic!("Unexpected error: {}", err),
            None => panic!("Expected Some"),
        }

        assert!(result.get(1).unwrap().is_err());

        match result.get(2) {
            Some(Ok(event)) => assert_eq!(
                *event,
                Event::Deposit {
                    client_id: 2,
                    transaction_id: 2,
                    amount: 2,
                }
            ),
            Some(Err(err)) => panic!("Unexpected error: {}", err),
            None => panic!("Expected Some"),
        }
    }

    #[test]
    fn test_to_events_iter_unknown_type() {
        let input = String::from("type,client,tx,amount\nunknown,1,1,1\n");
        let events_iter = to_events_iter(input.as_bytes());
        let result = events_iter.collect::<Vec<_>>();
        assert_eq!(result.len(), 1);

        match result.first() {
            Some(Err(err)) => assert_eq!(err.to_string(), "Unknown event kind: unknown."),
            Some(Ok(_)) => panic!("Expected failed event parse"),
            None => panic!("Expected Some"),
        };
    }

    #[test]
    fn test_to_events_iter_missing_amount() {
        let input = String::from("type,client,tx,amount\ndeposit,1,1,\n");
        let events_iter = to_events_iter(input.as_bytes());
        let result = events_iter.collect::<Vec<_>>();
        assert_eq!(result.len(), 1);

        match result.first() {
            Some(Err(err)) => assert_eq!(err.to_string(), "Missing amount."),
            Some(Ok(_)) => panic!("Expected failed event parse"),
            None => panic!("Expected Some"),
        };
    }

    #[test]
    fn test_write_results() {
        let mut writer = Vec::new();
        let result = HashMap::from([
            (
                1,
                Client {
                    held: 20,
                    total: 100,
                    locked: true,
                },
            ),
            (
                2,
                Client {
                    held: 6,
                    total: 7,
                    locked: false,
                },
            ),
        ]);

        write_result(result, &mut writer).expect("Expected no errors.");

        let output = String::from_utf8(writer).expect("Not UTF-8");
        assert_eq!(
            output,
            "client,available,held,total,locked\n1,80,20,100,true\n2,1,6,7,false\n"
        );
    }
}

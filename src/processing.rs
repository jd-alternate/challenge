use crate::client::Client;
use crate::types::Amount;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;

type TransactionID = u32;
pub type ClientID = u16;

// TODO: see if we can combine some fields here
enum Transaction {
    Deposit {
        client_id: ClientID,
        amount: Amount,
        // TODO: think if this needs to be a bool or an enum
        under_dispute: bool,
    },
    Withdrawal {
        client_id: ClientID,
        amount: Amount,
        under_dispute: bool,
    },
}

// TODO: note that it's unfortunate we've got CSV specific serde stuff here
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    Deposit {
        #[serde(rename = "tx")]
        transaction_id: TransactionID,
        #[serde(rename = "client")]
        client_id: ClientID,
        amount: Amount,
    },
    Withdrawal {
        #[serde(rename = "tx")]
        transaction_id: TransactionID,
        #[serde(rename = "client")]
        client_id: ClientID,
        amount: Amount,
    },
    Dispute {
        #[serde(rename = "tx")]
        transaction_id: TransactionID,
        #[serde(rename = "client")]
        client_id: ClientID,
    },
    Resolve {
        #[serde(rename = "tx")]
        transaction_id: TransactionID,
        #[serde(rename = "client")]
        client_id: ClientID,
    },
    Chargeback {
        #[serde(rename = "tx")]
        transaction_id: TransactionID,
        #[serde(rename = "client")]
        client_id: ClientID,
    },
}

// I really want to receive an iterator that gives me actual events. That will be much easier to test with.
pub fn process_events(
    events: impl Iterator<Item = Result<Event, Box<dyn Error>>>,
) -> Result<HashMap<ClientID, Client>, Box<dyn Error>> {
    let mut transactions_by_id = HashMap::new();
    let mut clients_by_id = HashMap::new();
    for result in events {
        let event = result?;
        match event {
            Event::Deposit {
                transaction_id,
                client_id,
                amount,
            } => {
                clients_by_id
                    .entry(client_id)
                    .or_insert_with(|| Client::new())
                    .deposit(amount);
                transactions_by_id.insert(
                    transaction_id,
                    Transaction::Deposit {
                        client_id,
                        amount,
                        under_dispute: false,
                    },
                );
            }
            _ => {}
        }
    }

    Ok(clients_by_id)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_empty() {
        // just going to pass an empty reader
        let input_events = vec![];
        let result = process_events(input_events.into_iter()).expect("failed to process events");
        assert_eq!(result, HashMap::new());
    }

    #[test]
    fn test_single_event() {
        // just going to pass an empty reader
        let input_events = vec![Ok(Event::Deposit {
            client_id: 1,
            transaction_id: 1,
            amount: 100,
        })];
        let result = process_events(input_events.into_iter()).expect("failed to process events");
        assert_eq!(
            result,
            HashMap::from([(
                1,
                Client {
                    held: 0,
                    total: 100,
                    locked: false
                }
            )])
        );
    }
}

use crate::client::Client;
use crate::types::Amount;
use crate::types::ClientID;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;

type TransactionID = u32;

enum TransactionKind {
    Deposit,
    Withdrawal { successful: bool },
}

// TODO: consider making fields readonly that shouldn't change
struct Transaction {
    client_id: ClientID,
    amount: Amount,
    under_dispute: bool,
    kind: TransactionKind,
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

struct Processor {
    clients_by_id: HashMap<ClientID, Client>,
    transactions_by_id: HashMap<TransactionID, Transaction>,
}

impl Processor {
    fn new() -> Self {
        Self {
            clients_by_id: HashMap::new(),
            transactions_by_id: HashMap::new(),
        }
    }

    fn process_event(&mut self, event: Event) -> Result<(), String> {
        match event {
            Event::Deposit {
                transaction_id,
                client_id,
                amount,
            } => self.deposit(transaction_id, client_id, amount),
            Event::Withdrawal {
                transaction_id,
                client_id,
                amount,
            } => self.withdraw(transaction_id, client_id, amount)?,
            Event::Dispute {
                transaction_id,
                client_id,
            } => self.dispute(transaction_id, client_id)?,
            Event::Resolve {
                transaction_id,
                client_id,
            } => self.resolve(transaction_id, client_id)?,
            _ => {}
        }

        Ok(())
    }

    fn deposit(&mut self, transaction_id: TransactionID, client_id: ClientID, amount: Amount) {
        let client = self.find_or_create_client(client_id);
        client.deposit(amount);
        self.create_transaction(
            transaction_id,
            Transaction {
                client_id,
                amount,
                under_dispute: false,
                kind: TransactionKind::Deposit,
            },
        )
    }

    // Arguably, instead of storing the transaction as unsuccessful you could just
    // not store it, but then you'd get a less useful error message upon the unlikely
    // event that a dispute is attempted.
    fn withdraw(
        &mut self,
        transaction_id: TransactionID,
        client_id: ClientID,
        amount: Amount,
    ) -> Result<(), String> {
        let client = self.find_or_create_client(client_id);
        let successful = client.withdraw(amount);
        self.create_transaction(
            transaction_id,
            Transaction {
                client_id,
                amount,
                under_dispute: false,
                kind: TransactionKind::Withdrawal { successful },
            },
        );

        if successful {
            Ok(())
        } else {
            Err(String::from("Insufficient funds."))
        }
    }

    fn dispute(
        &mut self,
        transaction_id: TransactionID,
        client_id: ClientID,
    ) -> Result<(), String> {
        let (transaction, client) = self.get_transaction_and_client(transaction_id)?;

        if transaction.under_dispute {
            return Err(format!(
                "Transaction {} is already under dispute.",
                transaction_id
            ));
        }

        // assuming that a client can only dispute their own transactions
        if client_id != transaction.client_id {
            return Err(format!(
                "Client id {} does not match transaction client id {}.",
                client_id, transaction.client_id
            ));
        }

        match transaction.kind {
            TransactionKind::Deposit => {
                client.hold(transaction.amount);
            }
            TransactionKind::Withdrawal { successful } => {
                if !successful {
                    return Err(format!(
                        "Original withdrawal ({}) was not successful, so it cannot be disputed.",
                        transaction_id
                    ));
                }

                client.hold(-1 * transaction.amount);
            }
        };

        transaction.under_dispute = true;

        Ok(())
    }

    fn resolve(
        &mut self,
        transaction_id: TransactionID,
        client_id: ClientID,
    ) -> Result<(), String> {
        let (transaction, client) = self.get_transaction_and_client(transaction_id)?;

        if !transaction.under_dispute {
            return Err(format!(
                "Transaction {} is not under dispute.",
                transaction_id
            ));
        }

        // assuming that a client can only dispute their own transactions
        if client_id != transaction.client_id {
            return Err(format!(
                "client id {} does not match transaction client id {}.",
                client_id, transaction.client_id
            ));
        }

        match transaction.kind {
            TransactionKind::Deposit => {
                client.hold(-1 * transaction.amount);
            }
            // ignoring whether withdrawal was successful given we can't dispute
            // unsuccessful withdrawals in the first place
            TransactionKind::Withdrawal { .. } => {
                client.hold(transaction.amount);
            }
        };

        transaction.under_dispute = false;

        Ok(())
    }

    fn find_or_create_client(&mut self, client_id: ClientID) -> &mut Client {
        self.clients_by_id
            .entry(client_id)
            .or_insert_with(|| Client::new(client_id))
    }

    fn create_transaction(&mut self, transaction_id: TransactionID, transaction: Transaction) {
        // if we already have a transaction with this ID, we will ignore the request
        // TODO: consider error-ing
        if self.transactions_by_id.contains_key(&transaction_id) {
            return;
        }

        self.transactions_by_id.insert(transaction_id, transaction);
    }

    fn get_transaction_and_client(
        &mut self,
        transaction_id: TransactionID,
    ) -> Result<(&mut Transaction, &mut Client), String> {
        let transaction = self
            .transactions_by_id
            .get_mut(&transaction_id)
            .ok_or(format!("Transaction {} not found.", transaction_id))?;

        let client = self
            .clients_by_id
            .get_mut(&transaction.client_id)
            .ok_or(format!("Client {} does not exist.", transaction.client_id))?;

        Ok((transaction, client))
    }
}

pub fn process_events(
    events: impl Iterator<Item = Result<Event, Box<dyn Error>>>,
) -> Result<(HashMap<ClientID, Client>, Vec<String>), Box<dyn Error>> {
    let mut processor = Processor::new();
    let mut errors = vec![];

    for result in events {
        let event = result?;
        let result = processor.process_event(event);
        if let Err(e) = result {
            errors.push(e)
        }
    }

    Ok((processor.clients_by_id, errors))
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_parse_empty() {
        let input_events = vec![];
        let result = process_events(input_events.into_iter()).expect("failed to process events");
        assert_eq!(result.0, HashMap::new());
        assert_eq!(result.1, Vec::<String>::new());
    }

    #[test]
    fn test_single_deposit() {
        let client_id = 1;
        let deposit_amount = 100;
        let input_events = vec![Ok(Event::Deposit {
            client_id: client_id,
            transaction_id: 1,
            amount: deposit_amount,
        })];

        let result = process_events(input_events.into_iter()).expect("failed to process events");

        assert_eq!(
            result.0,
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount,
                    locked: false
                }
            )])
        );

        assert_eq!(result.1, Vec::<String>::new());
    }

    #[test]
    fn test_two_deposits() {
        let client_id = 1;
        let first_deposit_amount = 100;
        let second_deposit_amount = 200;
        let input_events = vec![
            Ok(Event::Deposit {
                client_id: client_id,
                transaction_id: 1,
                amount: first_deposit_amount,
            }),
            Ok(Event::Deposit {
                client_id: client_id,
                transaction_id: 2,
                amount: second_deposit_amount,
            }),
        ];

        let result = process_events(input_events.into_iter()).expect("failed to process events");

        assert_eq!(
            result.0,
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: first_deposit_amount + second_deposit_amount,
                    locked: false
                }
            )])
        );

        assert_eq!(result.1, Vec::<String>::new());
    }

    #[test]
    fn test_unsuccessful_withdrawal() {
        let client_id = 1;
        let deposit_amount = 100;
        let withdrawal_amount = 120;
        let input_events = vec![
            Ok(Event::Deposit {
                client_id: client_id,
                transaction_id: 1,
                amount: deposit_amount,
            }),
            Ok(Event::Withdrawal {
                client_id: client_id,
                transaction_id: 2,
                amount: withdrawal_amount,
            }),
        ];

        let result = process_events(input_events.into_iter()).expect("failed to process events");

        assert_eq!(
            result.0,
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount,
                    locked: false
                }
            )])
        );

        assert_eq!(result.1, vec![String::from("Insufficient funds.")]);
    }

    #[test]
    fn test_successful_withdrawal() {
        let client_id = 1;
        let deposit_amount = 100;
        let withdrawal_amount = 20;
        let input_events = vec![
            Ok(Event::Deposit {
                client_id: client_id,
                transaction_id: 1,
                amount: deposit_amount,
            }),
            Ok(Event::Withdrawal {
                client_id: client_id,
                transaction_id: 2,
                amount: withdrawal_amount,
            }),
        ];

        let result = process_events(input_events.into_iter()).expect("failed to process events");

        assert_eq!(
            result.0,
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount - withdrawal_amount,
                    locked: false
                }
            )])
        );
        assert_eq!(result.1, Vec::<String>::new());
    }

    #[test]
    fn test_successful_disputed_deposit() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;
        let input_events = vec![
            Ok(Event::Deposit {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
                amount: deposit_amount,
            }),
            Ok(Event::Dispute {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
            }),
        ];

        let result = process_events(input_events.into_iter()).expect("failed to process events");

        assert_eq!(
            result.0,
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: deposit_amount,
                    total: deposit_amount,
                    locked: false
                }
            )])
        );
        assert_eq!(result.1, Vec::<String>::new());
    }

    #[test]
    fn test_unsuccessful_disputed_deposit_due_to_not_found_transaction() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;
        let input_events = vec![
            Ok(Event::Deposit {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
                amount: deposit_amount,
            }),
            Ok(Event::Dispute {
                client_id: client_id,
                transaction_id: 3,
            }),
        ];

        let result = process_events(input_events.into_iter()).expect("failed to process events");

        assert_eq!(
            result.0,
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount,
                    locked: false
                }
            )])
        );

        assert_eq!(result.1, vec![String::from("Transaction 3 not found.")]);
    }

    #[test]
    fn test_unsuccessful_disputed_deposit_due_to_mismatched_client() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;
        let input_events = vec![
            Ok(Event::Deposit {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
                amount: deposit_amount,
            }),
            Ok(Event::Dispute {
                client_id: 3,
                transaction_id: deposit_transaction_id,
            }),
        ];

        let result = process_events(input_events.into_iter()).expect("failed to process events");

        assert_eq!(
            result.0,
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount,
                    locked: false
                }
            )])
        );

        assert_eq!(
            result.1,
            vec![String::from(
                "Client id 3 does not match transaction client id 1.",
            )],
        );
    }

    #[test]
    fn test_unsuccessful_disputed_deposit_due_to_already_disputed() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;
        let input_events = vec![
            Ok(Event::Deposit {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
                amount: deposit_amount,
            }),
            Ok(Event::Dispute {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
            }),
            Ok(Event::Dispute {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
            }),
        ];

        let result = process_events(input_events.into_iter()).expect("failed to process events");

        assert_eq!(
            result.0,
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: deposit_amount,
                    total: deposit_amount,
                    locked: false
                }
            )])
        );

        assert_eq!(
            result.1,
            vec![String::from("Transaction 2 is already under dispute.")],
        );
    }

    #[test]
    fn test_successful_disputed_deposit_after_resolved() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;
        let input_events = vec![
            Ok(Event::Deposit {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
                amount: deposit_amount,
            }),
            Ok(Event::Dispute {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
            }),
            Ok(Event::Resolve {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
            }),
            Ok(Event::Dispute {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
            }),
        ];

        let result = process_events(input_events.into_iter()).expect("failed to process events");

        assert_eq!(
            result.0,
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: deposit_amount,
                    total: deposit_amount,
                    locked: false
                }
            )])
        );

        assert_eq!(result.1, Vec::<String>::new());
    }

    #[test]
    fn test_unsuccessful_disputed_withdrawal_due_to_unsuccessful_withdrawal() {
        let client_id = 1;
        let withdrawal_transaction_id = 2;
        let input_events = vec![
            Ok(Event::Withdrawal {
                client_id: client_id,
                transaction_id: withdrawal_transaction_id,
                amount: 10,
            }),
            Ok(Event::Dispute {
                client_id: client_id,
                transaction_id: withdrawal_transaction_id,
            }),
        ];

        let result = process_events(input_events.into_iter()).expect("failed to process events");

        assert_eq!(
            result.0,
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: 0,
                    locked: false
                }
            )])
        );

        assert_eq!(
            result.1,
            vec![
                String::from("Insufficient funds."),
                String::from(
                    "Original withdrawal (2) was not successful, so it cannot be disputed."
                )
            ],
        );
    }

    #[test]
    fn test_successful_resolved_dispute() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;
        let input_events = vec![
            Ok(Event::Deposit {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
                amount: deposit_amount,
            }),
            Ok(Event::Dispute {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
            }),
            Ok(Event::Resolve {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
            }),
        ];

        let result = process_events(input_events.into_iter()).expect("failed to process events");

        assert_eq!(
            result.0,
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount,
                    locked: false
                }
            )])
        );
        assert_eq!(result.1, Vec::<String>::new());
    }

    #[test]
    fn test_unsuccessful_resolved_dispute_due_to_lack_of_dispute() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;
        let input_events = vec![
            Ok(Event::Deposit {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
                amount: deposit_amount,
            }),
            Ok(Event::Resolve {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
            }),
        ];

        let result = process_events(input_events.into_iter()).expect("failed to process events");

        assert_eq!(
            result.0,
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount,
                    locked: false
                }
            )])
        );
        assert_eq!(
            result.1,
            vec![String::from("Transaction 2 is not under dispute.")]
        );
    }

    #[test]
    fn test_unsuccessful_resolved_dispute_due_to_double_resolve() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;
        let input_events = vec![
            Ok(Event::Deposit {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
                amount: deposit_amount,
            }),
            Ok(Event::Dispute {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
            }),
            Ok(Event::Resolve {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
            }),
            Ok(Event::Resolve {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
            }),
        ];

        let result = process_events(input_events.into_iter()).expect("failed to process events");

        assert_eq!(
            result.0,
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount,
                    locked: false
                }
            )])
        );
        assert_eq!(
            result.1,
            vec![String::from("Transaction 2 is not under dispute.")]
        );
    }

    #[test]
    fn test_unsuccessful_resolved_dispute_due_to_transaction_not_found() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;
        let input_events = vec![
            Ok(Event::Deposit {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
                amount: deposit_amount,
            }),
            Ok(Event::Dispute {
                client_id: client_id,
                transaction_id: deposit_transaction_id,
            }),
            Ok(Event::Resolve {
                client_id: client_id,
                transaction_id: 3,
            }),
        ];

        let result = process_events(input_events.into_iter()).expect("failed to process events");

        assert_eq!(
            result.0,
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: deposit_amount,
                    total: deposit_amount,
                    locked: false
                }
            )])
        );
        assert_eq!(result.1, vec![String::from("Transaction 3 not found.")]);
    }
}

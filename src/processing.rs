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

enum DisputeStatus {
    None, // if a dispute is resolves, we go back to this state
    Pending,
    ChargedBack,
}

// TODO: consider making fields readonly that shouldn't change
struct Transaction {
    client_id: ClientID,
    amount: Amount,
    kind: TransactionKind,
    dispute_status: DisputeStatus,
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
            } => self.withdraw(transaction_id, client_id, amount),
            Event::Dispute {
                transaction_id,
                client_id,
            } => self.dispute(transaction_id, client_id),
            Event::Resolve {
                transaction_id,
                client_id,
            } => self.resolve(transaction_id, client_id),
            Event::Chargeback {
                transaction_id,
                client_id,
            } => self.chargeback(transaction_id, client_id),
        }
    }

    fn deposit(
        &mut self,
        transaction_id: TransactionID,
        client_id: ClientID,
        amount: Amount,
    ) -> Result<(), String> {
        self.check_transaction_does_not_exist(transaction_id)?;

        let client = self.find_or_create_client(client_id);
        client.deposit(amount);
        self.create_transaction(
            transaction_id,
            Transaction {
                client_id,
                amount,
                dispute_status: DisputeStatus::None,
                kind: TransactionKind::Deposit,
            },
        );

        Ok(())
    }

    fn check_transaction_does_not_exist(
        &self,
        transaction_id: TransactionID,
    ) -> Result<(), String> {
        if self.transactions_by_id.contains_key(&transaction_id) {
            return Err(format!(
                "Transaction already exists with id {}.",
                transaction_id,
            ));
        }

        Ok(())
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
        self.check_transaction_does_not_exist(transaction_id)?;

        let client = self.find_or_create_client(client_id);
        let successful = client.withdraw(amount);
        self.create_transaction(
            transaction_id,
            Transaction {
                client_id,
                amount,
                dispute_status: DisputeStatus::None,
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
        Self::check_client_owns_transaction(client_id, transaction)?;

        match transaction.dispute_status {
            DisputeStatus::Pending => {
                return Err(format!(
                    "Transaction {} is already under dispute.",
                    transaction_id
                ));
            }
            DisputeStatus::ChargedBack => {
                return Err(format!(
                    "Transaction {} has already been charged back.",
                    transaction_id
                ));
            }
            DisputeStatus::None => {}
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

        transaction.dispute_status = DisputeStatus::Pending;

        Ok(())
    }

    fn resolve(
        &mut self,
        transaction_id: TransactionID,
        client_id: ClientID,
    ) -> Result<(), String> {
        let (transaction, client) = self.get_transaction_and_client(transaction_id)?;
        Self::check_client_owns_transaction(client_id, transaction)?;

        if !matches!(transaction.dispute_status, DisputeStatus::Pending) {
            return Err(format!(
                "Transaction {} is not under dispute.",
                transaction_id
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

        transaction.dispute_status = DisputeStatus::None;

        Ok(())
    }

    fn chargeback(
        &mut self,
        transaction_id: TransactionID,
        client_id: ClientID,
    ) -> Result<(), String> {
        let (transaction, client) = self.get_transaction_and_client(transaction_id)?;
        Self::check_client_owns_transaction(client_id, transaction)?;

        if !matches!(transaction.dispute_status, DisputeStatus::Pending) {
            return Err(format!(
                "Transaction {} is not under dispute.",
                transaction_id
            ));
        }

        match transaction.kind {
            TransactionKind::Deposit => {
                client.chargeback(transaction.amount);
            }
            // ignoring whether withdrawal was successful given we can't dispute
            // unsuccessful withdrawals in the first place
            TransactionKind::Withdrawal { .. } => {
                client.chargeback(-1 * transaction.amount);
            }
        };

        transaction.dispute_status = DisputeStatus::ChargedBack;

        Ok(())
    }

    fn check_client_owns_transaction(
        client_id: ClientID,
        transaction: &Transaction,
    ) -> Result<(), String> {
        if client_id != transaction.client_id {
            return Err(format!(
                "Client id {} does not match transaction client id {}.",
                client_id, transaction.client_id
            ));
        }

        Ok(())
    }

    fn find_or_create_client(&mut self, client_id: ClientID) -> &mut Client {
        self.clients_by_id
            .entry(client_id)
            .or_insert_with(|| Client::new(client_id))
    }

    fn create_transaction(&mut self, transaction_id: TransactionID, transaction: Transaction) {
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

    // helper method for when we just want to provide an input and assert on the output
    fn assert_results(
        input_events: Vec<Result<Event, Box<dyn Error>>>,
        expected_clients_by_id: HashMap<ClientID, Client>,
        expected_errors: Vec<String>,
    ) {
        let result = process_events(input_events.into_iter())
            .expect("Unexpectedly failed to process events.");

        assert_eq!(result.0, expected_clients_by_id);
        assert_eq!(result.1, expected_errors);
    }

    #[test]
    fn test_empty_input() {
        assert_results(vec![], HashMap::new(), vec![]);
    }

    #[test]
    fn test_single_deposit() {
        let client_id = 1;
        let deposit_amount = 100;

        assert_results(
            vec![Ok(Event::Deposit {
                client_id: client_id,
                transaction_id: 1,
                amount: deposit_amount,
            })],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount,
                    locked: false,
                },
            )]),
            vec![],
        );
    }

    #[test]
    fn test_two_deposits() {
        let client_id = 1;
        let first_deposit_amount = 100;
        let second_deposit_amount = 200;

        assert_results(
            vec![
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
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: first_deposit_amount + second_deposit_amount,
                    locked: false,
                },
            )]),
            vec![],
        );
    }

    #[test]
    fn test_unsuccessful_deposit_due_to_existing_transaction() {
        let client_id = 1;
        let transaction_id = 1;
        let first_deposit_amount = 10;

        assert_results(
            vec![
                Ok(Event::Deposit {
                    client_id: client_id,
                    transaction_id: transaction_id,
                    amount: first_deposit_amount,
                }),
                Ok(Event::Deposit {
                    client_id: client_id,
                    transaction_id: transaction_id,
                    amount: 20,
                }),
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: first_deposit_amount,
                    locked: false,
                },
            )]),
            vec![String::from("Transaction already exists with id 1.")],
        );
    }

    #[test]
    fn test_error_event() {
        let client_id = 1;
        let deposit_amount = 100;
        let input_events = vec![
            Ok(Event::Deposit {
                client_id: client_id,
                transaction_id: 1,
                amount: deposit_amount,
            }),
            Err("Test".into()),
            Ok(Event::Deposit {
                client_id: client_id,
                transaction_id: 2,
                amount: 10,
            }),
        ];

        let result = process_events(input_events.into_iter());

        assert!(result.is_err());
    }

    #[test]
    fn test_successful_withdrawal() {
        let client_id = 1;
        let deposit_amount = 100;
        let withdrawal_amount = 20;

        assert_results(
            vec![
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
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount - withdrawal_amount,
                    locked: false,
                },
            )]),
            vec![],
        );
    }

    #[test]
    fn test_unsuccessful_withdrawal_due_to_insufficient_funds() {
        let client_id = 1;
        let deposit_amount = 100;
        let withdrawal_amount = 120;

        assert_results(
            vec![
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
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount,
                    locked: false,
                },
            )]),
            vec![String::from("Insufficient funds.")],
        );
    }

    #[test]
    fn test_unsuccessful_withdrawal_due_to_existing_transaction() {
        let client_id = 1;
        let deposit_amount = 100;

        assert_results(
            vec![
                Ok(Event::Deposit {
                    client_id: client_id,
                    transaction_id: 1,
                    amount: deposit_amount,
                }),
                Ok(Event::Withdrawal {
                    client_id: client_id,
                    transaction_id: 1,
                    amount: 100,
                }),
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount,
                    locked: false,
                },
            )]),
            vec![String::from("Transaction already exists with id 1.")],
        );
    }

    #[test]
    fn test_successful_disputed_deposit() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Deposit {
                    client_id: client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::Dispute {
                    client_id: client_id,
                    transaction_id: deposit_transaction_id,
                }),
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: deposit_amount,
                    total: deposit_amount,
                    locked: false,
                },
            )]),
            vec![],
        );
    }

    #[test]
    fn test_unsuccessful_disputed_deposit_due_to_not_found_transaction() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Deposit {
                    client_id: client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::Dispute {
                    client_id: client_id,
                    transaction_id: 3,
                }),
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount,
                    locked: false,
                },
            )]),
            vec![String::from("Transaction 3 not found.")],
        );
    }

    #[test]
    fn test_unsuccessful_disputed_deposit_due_to_mismatched_client() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Deposit {
                    client_id: client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::Dispute {
                    client_id: 3,
                    transaction_id: deposit_transaction_id,
                }),
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount,
                    locked: false,
                },
            )]),
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

        assert_results(
            vec![
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
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: deposit_amount,
                    total: deposit_amount,
                    locked: false,
                },
            )]),
            vec![String::from("Transaction 2 is already under dispute.")],
        );
    }

    #[test]
    fn test_unsuccessful_disputed_deposit_due_to_already_charged_back() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Deposit {
                    client_id: client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::Dispute {
                    client_id: client_id,
                    transaction_id: deposit_transaction_id,
                }),
                Ok(Event::Chargeback {
                    client_id: client_id,
                    transaction_id: deposit_transaction_id,
                }),
                Ok(Event::Dispute {
                    client_id: client_id,
                    transaction_id: deposit_transaction_id,
                }),
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: 0,
                    locked: true,
                },
            )]),
            vec![String::from("Transaction 2 has already been charged back.")],
        );
    }

    #[test]
    fn test_successful_disputed_deposit_after_resolved() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;

        assert_results(
            vec![
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
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: deposit_amount,
                    total: deposit_amount,
                    locked: false,
                },
            )]),
            Vec::<String>::new(),
        );
    }

    #[test]
    fn test_unsuccessful_disputed_withdrawal_due_to_unsuccessful_withdrawal() {
        let client_id = 1;
        let withdrawal_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Withdrawal {
                    client_id: client_id,
                    transaction_id: withdrawal_transaction_id,
                    amount: 10,
                }),
                Ok(Event::Dispute {
                    client_id: client_id,
                    transaction_id: withdrawal_transaction_id,
                }),
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: 0,
                    locked: false,
                },
            )]),
            vec![
                String::from("Insufficient funds."),
                String::from(
                    "Original withdrawal (2) was not successful, so it cannot be disputed.",
                ),
            ],
        );
    }

    #[test]
    fn test_successful_resolved_deposit_dispute() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;

        assert_results(
            vec![
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
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount,
                    locked: false,
                },
            )]),
            Vec::<String>::new(),
        );
    }

    #[test]
    fn test_successful_resolved_withdrawal_dispute() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;
        let withdrawal_amount = 20;
        let withdrawal_transaction_id = 3;

        assert_results(
            vec![
                Ok(Event::Deposit {
                    client_id: client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::Withdrawal {
                    client_id: client_id,
                    transaction_id: withdrawal_transaction_id,
                    amount: withdrawal_amount,
                }),
                Ok(Event::Dispute {
                    client_id: client_id,
                    transaction_id: withdrawal_transaction_id,
                }),
                Ok(Event::Resolve {
                    client_id: client_id,
                    transaction_id: withdrawal_transaction_id,
                }),
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount - withdrawal_amount,
                    locked: false,
                },
            )]),
            Vec::<String>::new(),
        );
    }

    #[test]
    fn test_unsuccessful_resolved_dispute_due_to_lack_of_dispute() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Deposit {
                    client_id: client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::Resolve {
                    client_id: client_id,
                    transaction_id: deposit_transaction_id,
                }),
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount,
                    locked: false,
                },
            )]),
            vec![String::from("Transaction 2 is not under dispute.")],
        );
    }

    #[test]
    fn test_unsuccessful_resolved_dispute_due_to_double_resolve() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;

        assert_results(
            vec![
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
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount,
                    locked: false,
                },
            )]),
            vec![String::from("Transaction 2 is not under dispute.")],
        );
    }

    #[test]
    fn test_unsuccessful_resolved_dispute_due_to_transaction_not_found() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;

        assert_results(
            vec![
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
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: deposit_amount,
                    total: deposit_amount,
                    locked: false,
                },
            )]),
            vec![String::from("Transaction 3 not found.")],
        );
    }

    #[test]
    fn test_successful_deposit_chargeback() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Deposit {
                    client_id: client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::Dispute {
                    client_id: client_id,
                    transaction_id: deposit_transaction_id,
                }),
                Ok(Event::Chargeback {
                    client_id: client_id,
                    transaction_id: deposit_transaction_id,
                }),
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: 0,
                    locked: true,
                },
            )]),
            Vec::<String>::new(),
        );
    }

    #[test]
    fn test_successful_withdrawal_chargeback() {
        let client_id = 1;
        let deposit_transaction_id = 1;
        let deposit_amount = 100;
        let withdrawal_amount = 20;
        let withdrawal_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Deposit {
                    client_id: client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::Withdrawal {
                    client_id: client_id,
                    transaction_id: withdrawal_transaction_id,
                    amount: withdrawal_amount,
                }),
                Ok(Event::Dispute {
                    client_id: client_id,
                    transaction_id: withdrawal_transaction_id,
                }),
                Ok(Event::Chargeback {
                    client_id: client_id,
                    transaction_id: withdrawal_transaction_id,
                }),
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: 100,
                    locked: true,
                },
            )]),
            Vec::<String>::new(),
        );
    }

    #[test]
    fn test_unsuccessful_chargeback_due_to_not_disputed() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Deposit {
                    client_id: client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::Chargeback {
                    client_id: client_id,
                    transaction_id: deposit_transaction_id,
                }),
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount,
                    locked: false,
                },
            )]),
            vec![String::from("Transaction 2 is not under dispute.")],
        );
    }

    #[test]
    fn test_unsuccessful_chargeback_due_to_not_found_transaction() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Deposit {
                    client_id: client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::Chargeback {
                    client_id: client_id,
                    transaction_id: 3,
                }),
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount,
                    locked: false,
                },
            )]),
            vec![String::from("Transaction 3 not found.")],
        );
    }

    #[test]
    fn test_unsuccessful_chargeback_due_to_mismatched_client() {
        let client_id = 1;
        let deposit_amount = 100;
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Deposit {
                    client_id: client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::Chargeback {
                    client_id: 3,
                    transaction_id: deposit_transaction_id,
                }),
            ],
            HashMap::from([(
                client_id,
                Client {
                    id: client_id,
                    held: 0,
                    total: deposit_amount,
                    locked: false,
                },
            )]),
            vec![String::from(
                "Client id 3 does not match transaction client id 1.",
            )],
        );
    }
}

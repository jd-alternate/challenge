use super::processor::Processor;
use crate::model::{Client, ClientID, Event};

use std::{collections::HashMap, error::Error, io::Write};

// Takes an events iterator and processes each event. Returns the final state
// of the clients.
pub fn process_events(
    events_iter: impl Iterator<Item = Result<Event, Box<dyn Error>>>,
    error_logger: &mut impl Write,
) -> Result<HashMap<ClientID, Client>, Box<dyn Error>> {
    let mut processor = Processor::new();

    for event in events_iter {
        if let Err(e) = processor.process_event(event?) {
            error_logger.write_all(format!("{}\n", e).as_bytes())?;
        }
    }

    Ok(processor.clients_by_id())
}

#[cfg(test)]
mod test {
    use crate::model::{DisputeStepKind, TransactionKind};

    use super::*;
    use pretty_assertions::assert_eq;
    use rust_decimal_macros::dec;
    use std::io;

    // helper method for when we just want to provide an input and assert on the
    // output
    fn assert_results(
        input_events: Vec<Result<Event, Box<dyn Error>>>,
        expected_clients_by_id: HashMap<ClientID, Client>,
        expected_errors: Vec<String>,
    ) {
        let mut error_logger = Vec::new();

        let result = process_events(input_events.into_iter(), &mut error_logger)
            .expect("Unexpectedly failed to process events.");

        let error_str = String::from_utf8(error_logger).expect("Not UTF-8");
        let errors = error_str.lines().collect::<Vec<_>>();

        assert_eq!(expected_clients_by_id, result);
        assert_eq!(expected_errors, errors);
    }

    #[test]
    fn test_empty_input() {
        assert_results(vec![], HashMap::new(), vec![]);
    }

    #[test]
    fn test_single_deposit() {
        let client_id = 1;
        let deposit_amount = dec!(100);

        assert_results(
            vec![Ok(Event::Transaction {
                kind: TransactionKind::Deposit,
                client_id,
                transaction_id: 1,
                amount: deposit_amount,
            })],
            HashMap::from([(client_id, Client::from(dec!(0), deposit_amount, false))]),
            vec![],
        );
    }

    #[test]
    fn test_single_deposit_accurate() {
        let client_id = 1;
        let deposit_amount = dec!(100.12345);

        assert_results(
            vec![Ok(Event::Transaction {
                kind: TransactionKind::Deposit,
                client_id,
                transaction_id: 1,
                amount: deposit_amount,
            })],
            HashMap::from([(client_id, Client::from(dec!(0), deposit_amount, false))]),
            vec![],
        );
    }

    #[test]
    fn test_two_deposits() {
        let client_id = 1;
        let first_deposit_amount = dec!(100);
        let second_deposit_amount = dec!(200);

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: 1,
                    amount: first_deposit_amount,
                }),
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: 2,
                    amount: second_deposit_amount,
                }),
            ],
            HashMap::from([(
                client_id,
                Client::from(dec!(0), first_deposit_amount + second_deposit_amount, false),
            )]),
            vec![],
        );
    }

    #[test]
    fn test_unsuccessful_deposit_due_to_existing_transaction() {
        let client_id = 1;
        let transaction_id = 1;
        let first_deposit_amount = dec!(10);

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id,
                    amount: first_deposit_amount,
                }),
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id,
                    amount: dec!(20),
                }),
            ],
            HashMap::from([(
                client_id,
                Client::from(dec!(0), first_deposit_amount, false),
            )]),
            vec![String::from("Transaction already exists with id 1.")],
        );
    }

    #[test]
    fn test_error_event() {
        let client_id = 1;
        let deposit_amount = dec!(100);
        let input_events = vec![
            Ok(Event::Transaction {
                kind: TransactionKind::Deposit,
                client_id,
                transaction_id: 1,
                amount: deposit_amount,
            }),
            Err("Test".into()),
            Ok(Event::Transaction {
                kind: TransactionKind::Deposit,
                client_id,
                transaction_id: 2,
                amount: dec!(10),
            }),
        ];

        let result = process_events(input_events.into_iter(), &mut io::sink());

        assert!(result.is_err());
    }

    #[test]
    fn test_successful_withdrawal() {
        let client_id = 1;
        let deposit_amount = dec!(100);
        let withdrawal_amount = dec!(20);

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: 1,
                    amount: deposit_amount,
                }),
                Ok(Event::Transaction {
                    kind: TransactionKind::Withdrawal,
                    client_id,
                    transaction_id: 2,
                    amount: withdrawal_amount,
                }),
            ],
            HashMap::from([(
                client_id,
                Client::from(dec!(0), deposit_amount - withdrawal_amount, false),
            )]),
            vec![],
        );
    }

    #[test]
    fn test_unsuccessful_withdrawal_due_to_insufficient_funds() {
        let client_id = 1;
        let deposit_amount = dec!(100);
        let withdrawal_amount = dec!(120);

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: 1,
                    amount: deposit_amount,
                }),
                Ok(Event::Transaction {
                    kind: TransactionKind::Withdrawal,
                    client_id,
                    transaction_id: 2,
                    amount: withdrawal_amount,
                }),
            ],
            HashMap::from([(client_id, Client::from(dec!(0), deposit_amount, false))]),
            vec![String::from("Insufficient funds.")],
        );
    }

    #[test]
    fn test_unsuccessful_withdrawal_due_to_existing_transaction() {
        let client_id = 1;
        let deposit_amount = dec!(100);

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: 1,
                    amount: deposit_amount,
                }),
                Ok(Event::Transaction {
                    kind: TransactionKind::Withdrawal,
                    client_id,
                    transaction_id: 1,
                    amount: dec!(100),
                }),
            ],
            HashMap::from([(client_id, Client::from(dec!(0), deposit_amount, false))]),
            vec![String::from("Transaction already exists with id 1.")],
        );
    }

    #[test]
    fn test_successful_disputed_deposit() {
        let client_id = 1;
        let deposit_amount = dec!(100);
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Dispute,
                    client_id,
                    transaction_id: deposit_transaction_id,
                }),
            ],
            HashMap::from([(
                client_id,
                Client::from(deposit_amount, deposit_amount, false),
            )]),
            vec![],
        );
    }

    #[test]
    fn test_unsuccessful_disputed_deposit_due_to_not_found_transaction() {
        let client_id = 1;
        let deposit_amount = dec!(100);
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Dispute,
                    client_id,
                    transaction_id: 3,
                }),
            ],
            HashMap::from([(client_id, Client::from(dec!(0), deposit_amount, false))]),
            vec![String::from("Transaction 3 not found.")],
        );
    }

    #[test]
    fn test_unsuccessful_disputed_deposit_due_to_mismatched_client() {
        let client_id = 1;
        let deposit_amount = dec!(100);
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Dispute,
                    client_id: 3,
                    transaction_id: deposit_transaction_id,
                }),
            ],
            HashMap::from([(client_id, Client::from(dec!(0), deposit_amount, false))]),
            vec![String::from(
                "Client id 3 does not match transaction client id 1.",
            )],
        );
    }

    #[test]
    fn test_unsuccessful_disputed_deposit_due_to_already_disputed() {
        let client_id = 1;
        let deposit_amount = dec!(100);
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Dispute,
                    client_id,
                    transaction_id: deposit_transaction_id,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Dispute,
                    client_id,
                    transaction_id: deposit_transaction_id,
                }),
            ],
            HashMap::from([(
                client_id,
                Client::from(deposit_amount, deposit_amount, false),
            )]),
            vec![String::from("Transaction is already disputed.")],
        );
    }

    #[test]
    fn test_unsuccessful_disputed_deposit_due_to_already_charged_back() {
        let client_id = 1;
        let deposit_amount = dec!(100);
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Dispute,
                    client_id,
                    transaction_id: deposit_transaction_id,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Chargeback,
                    client_id,
                    transaction_id: deposit_transaction_id,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Dispute,
                    client_id,
                    transaction_id: deposit_transaction_id,
                }),
            ],
            HashMap::from([(client_id, Client::from(dec!(0), dec!(0), true))]),
            vec![String::from("Transaction has already been charged back.")],
        );
    }

    #[test]
    fn test_successful_disputed_deposit_after_resolved() {
        let client_id = 1;
        let deposit_amount = dec!(100);
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Dispute,
                    client_id,
                    transaction_id: deposit_transaction_id,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Resolve,
                    client_id,
                    transaction_id: deposit_transaction_id,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Dispute,
                    client_id,
                    transaction_id: deposit_transaction_id,
                }),
            ],
            HashMap::from([(
                client_id,
                Client::from(deposit_amount, deposit_amount, false),
            )]),
            Vec::<String>::new(),
        );
    }

    #[test]
    // worth verifying that we would not in fact create a transaction in this case.
    fn test_unsuccessful_disputed_withdrawal_due_to_unsuccessful_withdrawal() {
        let client_id = 1;
        let withdrawal_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Withdrawal,
                    client_id,
                    transaction_id: withdrawal_transaction_id,
                    amount: dec!(10),
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Dispute,
                    client_id,
                    transaction_id: withdrawal_transaction_id,
                }),
            ],
            HashMap::from([(client_id, Client::from(dec!(0), dec!(0), false))]),
            vec![
                String::from("Insufficient funds."),
                String::from("Transaction 2 not found."),
            ],
        );
    }

    #[test]
    fn test_successful_resolved_deposit_dispute() {
        let client_id = 1;
        let deposit_amount = dec!(100);
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Dispute,
                    client_id,
                    transaction_id: deposit_transaction_id,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Resolve,
                    client_id,
                    transaction_id: deposit_transaction_id,
                }),
            ],
            HashMap::from([(client_id, Client::from(dec!(0), deposit_amount, false))]),
            Vec::<String>::new(),
        );
    }

    #[test]
    fn test_successful_resolved_withdrawal_dispute() {
        let client_id = 1;
        let deposit_amount = dec!(100);
        let deposit_transaction_id = 2;
        let withdrawal_amount = dec!(20);
        let withdrawal_transaction_id = 3;

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::Transaction {
                    kind: TransactionKind::Withdrawal,
                    client_id,
                    transaction_id: withdrawal_transaction_id,
                    amount: withdrawal_amount,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Dispute,
                    client_id,
                    transaction_id: withdrawal_transaction_id,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Resolve,
                    client_id,
                    transaction_id: withdrawal_transaction_id,
                }),
            ],
            HashMap::from([(
                client_id,
                Client::from(dec!(0), deposit_amount - withdrawal_amount, false),
            )]),
            Vec::<String>::new(),
        );
    }

    #[test]
    fn test_unsuccessful_resolved_dispute_due_to_lack_of_dispute() {
        let client_id = 1;
        let deposit_amount = dec!(100);
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Resolve,
                    client_id,
                    transaction_id: deposit_transaction_id,
                }),
            ],
            HashMap::from([(client_id, Client::from(dec!(0), deposit_amount, false))]),
            vec![String::from("Transaction is not disputed.")],
        );
    }

    #[test]
    fn test_unsuccessful_resolved_dispute_due_to_double_resolve() {
        let client_id = 1;
        let deposit_amount = dec!(100);
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Dispute,
                    client_id,
                    transaction_id: deposit_transaction_id,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Resolve,
                    client_id,
                    transaction_id: deposit_transaction_id,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Resolve,
                    client_id,
                    transaction_id: deposit_transaction_id,
                }),
            ],
            HashMap::from([(client_id, Client::from(dec!(0), deposit_amount, false))]),
            vec![String::from("Transaction is not disputed.")],
        );
    }

    #[test]
    fn test_unsuccessful_resolved_dispute_due_to_transaction_not_found() {
        let client_id = 1;
        let deposit_amount = dec!(100);
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Dispute,
                    client_id,
                    transaction_id: deposit_transaction_id,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Resolve,
                    client_id,
                    transaction_id: 3,
                }),
            ],
            HashMap::from([(
                client_id,
                Client::from(deposit_amount, deposit_amount, false),
            )]),
            vec![String::from("Transaction 3 not found.")],
        );
    }

    #[test]
    fn test_successful_deposit_chargeback() {
        let client_id = 1;
        let deposit_amount = dec!(100);
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Dispute,
                    client_id,
                    transaction_id: deposit_transaction_id,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Chargeback,
                    client_id,
                    transaction_id: deposit_transaction_id,
                }),
            ],
            HashMap::from([(client_id, Client::from(dec!(0), dec!(0), true))]),
            Vec::<String>::new(),
        );
    }

    #[test]
    fn test_successful_withdrawal_chargeback() {
        let client_id = 1;
        let deposit_transaction_id = 1;
        let deposit_amount = dec!(100);
        let withdrawal_amount = dec!(20);
        let withdrawal_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::Transaction {
                    kind: TransactionKind::Withdrawal,
                    client_id,
                    transaction_id: withdrawal_transaction_id,
                    amount: withdrawal_amount,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Dispute,
                    client_id,
                    transaction_id: withdrawal_transaction_id,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Chargeback,
                    client_id,
                    transaction_id: withdrawal_transaction_id,
                }),
            ],
            HashMap::from([(client_id, Client::from(dec!(0), deposit_amount, true))]),
            Vec::<String>::new(),
        );
    }

    #[test]
    fn test_unsuccessful_chargeback_due_to_not_disputed() {
        let client_id = 1;
        let deposit_amount = dec!(100);
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Chargeback,
                    client_id,
                    transaction_id: deposit_transaction_id,
                }),
            ],
            HashMap::from([(client_id, Client::from(dec!(0), deposit_amount, false))]),
            vec![String::from("Transaction is not disputed.")],
        );
    }

    #[test]
    fn test_unsuccessful_chargeback_due_to_not_found_transaction() {
        let client_id = 1;
        let deposit_amount = dec!(100);
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Chargeback,
                    client_id,
                    transaction_id: 3,
                }),
            ],
            HashMap::from([(client_id, Client::from(dec!(0), deposit_amount, false))]),
            vec![String::from("Transaction 3 not found.")],
        );
    }

    #[test]
    fn test_unsuccessful_chargeback_due_to_mismatched_client() {
        let client_id = 1;
        let deposit_amount = dec!(100);
        let deposit_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Chargeback,
                    client_id: 3,
                    transaction_id: deposit_transaction_id,
                }),
            ],
            HashMap::from([(client_id, Client::from(dec!(0), deposit_amount, false))]),
            vec![String::from(
                "Client id 3 does not match transaction client id 1.",
            )],
        );
    }

    #[test]
    fn test_disputed_deposit_after_equivalent_withdrawal() {
        let client_id = 1;
        let deposit_amount = dec!(100);
        let deposit_transaction_id = 1;
        let withdrawal_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::Transaction {
                    kind: TransactionKind::Withdrawal,
                    client_id,
                    transaction_id: withdrawal_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Dispute,
                    client_id,
                    transaction_id: deposit_transaction_id,
                }),
            ],
            HashMap::from([(client_id, Client::from(deposit_amount, dec!(0), false))]),
            vec![],
        );
    }

    #[test]
    fn test_disputed_withdrawal_after_equivalent_deposit() {
        let client_id = 1;
        let deposit_amount = dec!(100);
        let deposit_transaction_id = 1;
        let withdrawal_transaction_id = 2;

        assert_results(
            vec![
                Ok(Event::Transaction {
                    kind: TransactionKind::Deposit,
                    client_id,
                    transaction_id: deposit_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::Transaction {
                    kind: TransactionKind::Withdrawal,
                    client_id,
                    transaction_id: withdrawal_transaction_id,
                    amount: deposit_amount,
                }),
                Ok(Event::DisputeStep {
                    kind: DisputeStepKind::Dispute,
                    client_id,
                    transaction_id: withdrawal_transaction_id,
                }),
            ],
            HashMap::from([(client_id, Client::from(deposit_amount, dec!(0), false))]),
            vec![],
        );
    }
}

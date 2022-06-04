use crate::model::{
    Amount, Client, ClientID, DisputeStatus, Event, Transaction, TransactionID, TransactionKind,
};

use std::collections::HashMap;

// This maintains the state of the system (clients and transactions) and
// processes new events.
pub struct Processor {
    pub clients_by_id: HashMap<ClientID, Client>,
    pub transactions_by_id: HashMap<TransactionID, Transaction>,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            clients_by_id: HashMap::new(),
            transactions_by_id: HashMap::new(),
        }
    }

    pub fn process_event(&mut self, event: Event) -> Result<(), String> {
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
        client.deposit(amount)?;
        self.create_transaction(
            transaction_id,
            Transaction::new(client_id, amount, TransactionKind::Deposit),
        );

        Ok(())
    }

    fn withdraw(
        &mut self,
        transaction_id: TransactionID,
        client_id: ClientID,
        amount: Amount,
    ) -> Result<(), String> {
        self.check_transaction_does_not_exist(transaction_id)?;

        let client = self.find_or_create_client(client_id);
        client.withdraw(amount)?;
        self.create_transaction(
            transaction_id,
            Transaction::new(client_id, amount, TransactionKind::Withdrawal),
        );

        Ok(())
    }

    fn dispute(
        &mut self,
        transaction_id: TransactionID,
        client_id: ClientID,
    ) -> Result<(), String> {
        let (transaction, client) = self.get_transaction_and_client(transaction_id)?;
        Self::check_client_owns_transaction(client_id, transaction)?;

        match transaction.dispute_status() {
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

        match transaction.kind() {
            TransactionKind::Deposit => {
                client.hold(transaction.amount());
            }
            TransactionKind::Withdrawal => {
                client.hold(-transaction.amount());
            }
        };

        transaction.set_dispute_status(DisputeStatus::Pending);

        Ok(())
    }

    fn resolve(
        &mut self,
        transaction_id: TransactionID,
        client_id: ClientID,
    ) -> Result<(), String> {
        let (transaction, client) = self.get_transaction_and_client(transaction_id)?;
        Self::check_client_owns_transaction(client_id, transaction)?;

        if !transaction.is_under_dispute() {
            return Err(format!(
                "Transaction {} is not under dispute.",
                transaction_id
            ));
        }

        match transaction.kind() {
            TransactionKind::Deposit => {
                client.hold(-transaction.amount());
            }

            TransactionKind::Withdrawal => {
                client.hold(transaction.amount());
            }
        };

        transaction.set_dispute_status(DisputeStatus::None);

        Ok(())
    }

    fn chargeback(
        &mut self,
        transaction_id: TransactionID,
        client_id: ClientID,
    ) -> Result<(), String> {
        let (transaction, client) = self.get_transaction_and_client(transaction_id)?;
        Self::check_client_owns_transaction(client_id, transaction)?;

        if !transaction.is_under_dispute() {
            return Err(format!(
                "Transaction {} is not under dispute.",
                transaction_id
            ));
        }

        match transaction.kind() {
            TransactionKind::Deposit => {
                client.chargeback(transaction.amount());
            }

            TransactionKind::Withdrawal => {
                client.chargeback(-transaction.amount());
            }
        };

        transaction.set_dispute_status(DisputeStatus::ChargedBack);

        Ok(())
    }

    fn check_client_owns_transaction(
        client_id: ClientID,
        transaction: &Transaction,
    ) -> Result<(), String> {
        if client_id != transaction.client_id() {
            return Err(format!(
                "Client id {} does not match transaction client id {}.",
                client_id,
                transaction.client_id()
            ));
        }

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

    fn find_or_create_client(&mut self, client_id: ClientID) -> &mut Client {
        self.clients_by_id
            .entry(client_id)
            .or_insert_with(Client::new)
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
            .get_mut(&transaction.client_id())
            .ok_or(format!(
                "Client {} does not exist.",
                transaction.client_id()
            ))?;

        Ok((transaction, client))
    }
}

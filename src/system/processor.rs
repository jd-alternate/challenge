use crate::model::{
    Amount, Client, ClientID, DisputeStatus, DisputeStepKind, Event, Transaction, TransactionID,
    TransactionKind,
};

use std::collections::HashMap;

// This maintains the state of the system (clients and transactions) and
// processes new events. We're not testing it directly because it's an
// implementation detail.
pub struct Processor {
    clients_by_id: HashMap<ClientID, Client>,
    transactions_by_id: HashMap<TransactionID, Transaction>,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            clients_by_id: HashMap::new(),
            transactions_by_id: HashMap::new(),
        }
    }

    // Expected to be called once all the events have been processed, hence taking
    // ownership of `self`.
    pub fn clients_by_id(self) -> HashMap<ClientID, Client> {
        self.clients_by_id
    }

    pub fn process_event(&mut self, event: Event) -> Result<(), String> {
        match event {
            Event::Transaction {
                kind,
                transaction_id,
                client_id,
                amount,
            } => match kind {
                TransactionKind::Deposit => self.deposit(transaction_id, client_id, amount),
                TransactionKind::Withdrawal => self.withdraw(transaction_id, client_id, amount),
            },
            Event::DisputeStep {
                kind,
                transaction_id,
                client_id,
            } => match kind {
                DisputeStepKind::Dispute => self.dispute(transaction_id, client_id),
                DisputeStepKind::Resolve => self.resolve(transaction_id, client_id),
                DisputeStepKind::Chargeback => self.chargeback(transaction_id, client_id),
            },
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

        transaction.validate_dispute_status_transition(DisputeStatus::Disputed)?;

        match transaction.kind() {
            TransactionKind::Deposit => {
                client.hold(transaction.amount());
            }
            TransactionKind::Withdrawal => {
                client.hold(-transaction.amount());
            }
        };

        transaction.set_dispute_status(DisputeStatus::Disputed);

        Ok(())
    }

    fn resolve(
        &mut self,
        transaction_id: TransactionID,
        client_id: ClientID,
    ) -> Result<(), String> {
        let (transaction, client) = self.get_transaction_and_client(transaction_id)?;
        Self::check_client_owns_transaction(client_id, transaction)?;

        transaction.validate_dispute_status_transition(DisputeStatus::None)?;

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

        transaction.validate_dispute_status_transition(DisputeStatus::ChargedBack)?;

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

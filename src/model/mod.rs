pub mod client;
pub use client::Client;

use rust_decimal::prelude::Decimal;

// A quick overview of the modelling here: we have a sequence of Events we need to
// process. Some events (deposits and withdrawals) create transactions, and other
// events (disputes/resolves/chargebacks) act on transactions. Any event can
// update the state of a Client, and every event is associated with one Client.

// Defining these type aliases so that we can easily update them if needed.
pub type Amount = Decimal;
pub type ClientID = u16;
pub type TransactionID = u32;

// Represents events in our system. These do not represent successfully processed events,
// but rather the events that need to be processed.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Event {
    Deposit {
        transaction_id: TransactionID,
        client_id: ClientID,
        amount: Amount,
    },
    Withdrawal {
        transaction_id: TransactionID,
        client_id: ClientID,
        amount: Amount,
    },
    Dispute {
        transaction_id: TransactionID,
        client_id: ClientID,
    },
    Resolve {
        transaction_id: TransactionID,
        client_id: ClientID,
    },
    Chargeback {
        transaction_id: TransactionID,
        client_id: ClientID,
    },
}

// Represents a transfer of money (either deposit or withdrawal). This does _not_
// represent disputes/resolutions: those are represented by events and act on transactions.
pub struct Transaction {
    client_id: ClientID,
    amount: Amount,
    kind: TransactionKind,
    dispute_status: DisputeStatus,
}

impl Transaction {
    pub fn new(client_id: ClientID, amount: Amount, kind: TransactionKind) -> Self {
        Self {
            client_id,
            amount,
            kind,
            dispute_status: DisputeStatus::None,
        }
    }

    pub fn client_id(&self) -> ClientID {
        self.client_id
    }

    pub fn amount(&self) -> Amount {
        self.amount
    }

    pub fn kind(&self) -> &TransactionKind {
        &self.kind
    }

    pub fn dispute_status(&self) -> &DisputeStatus {
        &self.dispute_status
    }

    pub fn set_dispute_status(&mut self, dispute_status: DisputeStatus) {
        self.dispute_status = dispute_status;
    }

    pub fn is_under_dispute(&self) -> bool {
        matches!(self.dispute_status(), DisputeStatus::Pending)
    }
}

pub enum TransactionKind {
    Deposit,
    Withdrawal,
}

pub enum DisputeStatus {
    None, // if a dispute is resolves, we go back to this state
    Pending,
    ChargedBack,
}

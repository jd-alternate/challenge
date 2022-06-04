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
    pub client_id: ClientID,
    pub amount: Amount,
    pub kind: TransactionKind,
    pub dispute_status: DisputeStatus,
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

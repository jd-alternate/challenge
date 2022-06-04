pub mod client;
pub mod transaction;
pub use client::*;
pub use transaction::*;

use rust_decimal::prelude::Decimal;

// A quick overview of the modelling here: we have a sequence of Events we need
// to process. Some events (deposits and withdrawals) create transactions, and
// other events (disputes/resolves/chargebacks) act on transactions. Any event
// can update the state of a Client, and every event is associated with one
// Client.

pub type Amount = Decimal;

// Represents events in our system. These do not represent successfully
// processed events, but rather the events that need to be processed.
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

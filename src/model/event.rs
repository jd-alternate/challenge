use super::{Amount, ClientID, TransactionID};

// Represents events in our system. These do not represent successfully
// processed events, but rather the events that need to be processed.
#[derive(Debug, PartialEq, Eq)]
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

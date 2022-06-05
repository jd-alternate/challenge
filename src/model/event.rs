use super::{Amount, ClientID, TransactionID, TransactionKind};

// Represents events in our system. These do not represent successfully
// processed events, but rather the events that need to be processed.
#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    Transaction {
        kind: TransactionKind,
        transaction_id: TransactionID,
        client_id: ClientID,
        amount: Amount,
    },
    DisputeStep {
        kind: DisputeStepKind,
        transaction_id: TransactionID,
        client_id: ClientID,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum DisputeStepKind {
    Dispute,
    Resolve,
    Chargeback,
}

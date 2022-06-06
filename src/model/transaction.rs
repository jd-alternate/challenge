use super::{Amount, ClientID};

pub type TransactionID = u32;

// Represents a transfer of money (either deposit or withdrawal). This does
// _not_ represent disputes/resolutions: those are represented by events and act
// on transactions.
pub struct Transaction {
    client_id: ClientID,
    amount: Amount,
    kind: TransactionKind,
    dispute_status: DisputeStatus,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TransactionKind {
    Deposit,
    Withdrawal,
}

pub enum DisputeStatus {
    Undisputed, // if a dispute is resolves, we go back to this state
    Disputed,
    ChargedBack,
}

use DisputeStatus::*;

impl Transaction {
    pub fn new(client_id: ClientID, amount: Amount, kind: TransactionKind) -> Self {
        Self {
            client_id,
            amount,
            kind,
            dispute_status: Undisputed,
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

    pub fn set_dispute_status(&mut self, dispute_status: DisputeStatus) {
        self.dispute_status = dispute_status;
    }

    pub fn validate_dispute_status_transition(
        &self,
        new_dispute_status: DisputeStatus,
    ) -> Result<(), String> {
        match (&self.dispute_status, new_dispute_status) {
            (Undisputed, Disputed) | (Disputed, Undisputed) | (Disputed, ChargedBack) => Ok(()),

            (ChargedBack, _) => Err(String::from("Transaction has already been charged back.")),
            (Undisputed, _) => Err(String::from("Transaction is not disputed.")),
            (Disputed, Disputed) => Err(String::from("Transaction is already disputed.")),
        }
    }
}

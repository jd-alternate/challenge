use crate::types::Amount;
use crate::types::ClientID;

#[derive(Debug, PartialEq)]
pub struct Client {
    pub id: ClientID,
    pub held: Amount,
    pub total: Amount,
    pub locked: bool,
}

impl Client {
    pub fn new(id: ClientID) -> Self {
        Self {
            id,
            held: 0,
            total: 0,
            locked: false,
        }
    }

    // TODO: consider having available actually stored so we don't need to do the calculation every time
    pub fn available(&self) -> Amount {
        self.total - self.held
    }

    pub fn deposit(&mut self, amount: Amount) {
        self.total += amount;
    }

    // TODO: should I mark the transaction as having failed?
    pub fn withdraw(&mut self, amount: Amount) -> bool {
        if self.available() < amount {
            false
        } else {
            self.total -= amount;
            true
        }
    }

    pub fn hold(&mut self, amount: Amount) {
        // TODO: can you hold negative money?
        self.held += amount;
    }

    pub fn chargeback(&mut self, amount: Amount) {
        self.held -= amount;
        self.total -= amount;
        self.locked = true;
    }
}

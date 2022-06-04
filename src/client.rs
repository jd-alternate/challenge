use crate::types::Amount;

// Represents the current state of a client account.
#[derive(Debug, PartialEq, Eq)]
// TODO: privatise fields
pub struct Client {
    pub held: Amount,
    pub total: Amount,
    // TODO: decide what to do if a client is locked
    pub locked: bool,
}

impl Client {
    pub fn new() -> Self {
        Self {
            held: 0,
            total: 0,
            locked: false,
        }
    }

    pub fn available(&self) -> Amount {
        self.total - self.held
    }

    pub fn deposit(&mut self, amount: Amount) {
        self.total += amount;
    }

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

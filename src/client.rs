use crate::types::Amount;

// currently getting a false positive 'unused import' error here
use rust_decimal_macros::dec;

// Represents the current state of a client account.
#[derive(Debug, PartialEq, Eq)]
// TODO: privatise fields
pub struct Client {
    pub held: Amount,
    pub total: Amount,
    pub locked: bool,
}

impl Client {
    pub fn new() -> Self {
        Self {
            held: dec!(0),
            total: dec!(0),
            locked: false,
        }
    }

    pub fn available(&self) -> Amount {
        self.total - self.held
    }

    pub fn deposit(&mut self, amount: Amount) -> Result<(), String> {
        if self.locked {
            return Err(String::from("Cannot deposit when account is locked."));
        }

        self.total += amount;
        Ok(())
    }

    pub fn withdraw(&mut self, amount: Amount) -> Result<(), String> {
        if self.locked {
            return Err(String::from("Cannot withdraw when account is locked."));
        }

        if self.available() < amount {
            return Err(String::from("Insufficient funds."));
        } else {
            self.total -= amount;
            return Ok(());
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

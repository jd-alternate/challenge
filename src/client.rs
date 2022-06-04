use crate::types::Amount;

// currently getting a false positive 'unused import' error here
use rust_decimal_macros::dec;

// Represents the current state of a client account.
#[derive(Debug, PartialEq, Eq)]
pub struct Client {
    held: Amount,
    total: Amount,
    locked: bool,
}

impl Client {
    pub fn new() -> Self {
        Self {
            held: dec!(0),
            total: dec!(0),
            locked: false,
        }
    }

    #[cfg(test)]
    pub fn from(held: Amount, total: Amount, locked: bool) -> Self {
        Self {
            held,
            total,
            locked,
        }
    }

    pub fn get_held(&self) -> Amount {
        self.held
    }

    pub fn get_total(&self) -> Amount {
        self.total
    }

    pub fn get_locked(&self) -> bool {
        self.locked
    }

    pub fn get_available(&self) -> Amount {
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

        if self.get_available() < amount {
            Err(String::from("Insufficient funds."))
        } else {
            self.total -= amount;
            Ok(())
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

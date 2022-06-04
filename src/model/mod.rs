pub mod client;
pub mod event;
pub mod transaction;
pub use client::*;
pub use event::*;
pub use transaction::*;

use rust_decimal::prelude::Decimal;

// A quick overview of the modelling here: we have a sequence of Events we need
// to process. Some events (deposits and withdrawals) create transactions, and
// other events (disputes/resolves/chargebacks) act on transactions. Any event
// can update the state of a Client, and every event is associated with one
// Client.

pub type Amount = Decimal;

use std::error::Error;

use challenge;
fn main() -> Result<(), Box<dyn Error>> {
    challenge::run()
}

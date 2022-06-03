// Everything CSV-related lives here.

use std::collections::HashMap;
use std::error::Error;
use std::io::{Read, Write};

use crate::client::Client;
use crate::processing::ClientID;
use crate::processing::Event;

// this will return an iterator which itself yields Event
pub fn csv_iterator(reader: impl Read) -> impl Iterator<Item = Result<Event, Box<dyn Error>>> {
    csv::Reader::from_reader(reader)
        .into_deserialize()
        .map(|result| result.map_err(|e| e.into()))
}

pub fn write_result(
    result: HashMap<ClientID, Client>,
    mut writer: impl Write,
) -> Result<(), Box<dyn Error>> {
    writer.write(b"client,available,held,total,locked")?;
    for (client_id, client) in result {
        writer.write(to_csv_row(client_id, &client).as_bytes())?;
    }

    Ok(())
}

// TODO: should this instead return direct bytes?
fn to_csv_row(client_id: ClientID, client: &Client) -> String {
    format!(
        "{},{},{},{},{}\n",
        client_id,
        client.available(),
        client.held,
        client.total,
        client.locked
    )
}

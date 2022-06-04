use serde::Serialize;
use std::{collections::HashMap, error::Error, io::Write};

use crate::model::{Amount, Client, ClientID};

// Intermediary representation of a client for serialization.
#[derive(Serialize)]
struct CsvClient {
    client: ClientID,
    available: Amount,
    held: Amount,
    total: Amount,
    locked: bool,
}

// Takes the resultant clients after processing events, and writes them to the
// given writer in CSV form.
pub fn write_report(
    clients_by_id: HashMap<ClientID, Client>,
    writer: impl Write,
) -> Result<(), Box<dyn Error>> {
    let csv_clients_iter = convert_to_csv_clients(clients_by_id);
    write_csv_clients(csv_clients_iter, writer)
}

fn convert_to_csv_clients(
    clients_by_id: HashMap<ClientID, Client>,
) -> impl Iterator<Item = CsvClient> {
    let mut entries: Vec<(ClientID, Client)> = clients_by_id.into_iter().collect();
    // This sorting is admittedly mostly for the sake of making testing easier,
    // though I assume that actually producing a report is a small part that happens
    // at the end of a long process of processing events, and I also assume that
    // it's convenient to order records by client ID despite the spec being
    // indifferent. If this assumption proves invalid we can ditch the sorting
    // and just update the test.
    entries.sort_by(|(a, _), (b, _)| a.cmp(b));
    entries
        .into_iter()
        .map(|(client_id, client)| csv_client_from_client(client_id, client))
}

fn write_csv_clients(
    csv_clients: impl Iterator<Item = CsvClient>,
    writer: impl Write,
) -> Result<(), Box<dyn Error>> {
    let mut wtr = csv::Writer::from_writer(writer);

    for client in csv_clients {
        wtr.serialize(client)?;
    }

    wtr.flush()?;

    Ok(())
}

fn csv_client_from_client(client_id: ClientID, client: Client) -> CsvClient {
    CsvClient {
        client: client_id,
        available: client.available(),
        held: client.held(),
        total: client.total(),
        locked: client.locked(),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;
    use rust_decimal_macros::dec;

    #[test]
    fn test_write_reports() {
        let mut writer = Vec::new();
        let result = HashMap::from([
            (1, Client::from(dec!(20), dec!(100), true)),
            (2, Client::from(dec!(6), dec!(7), false)),
        ]);

        write_report(result, &mut writer).expect("Expected no errors.");

        let output = String::from_utf8(writer).expect("Not UTF-8");
        assert_eq!(
            concat!(
                "client,available,held,total,locked\n",
                "1,80,20,100,true\n",
                "2,1,6,7,false\n"
            ),
            output,
        );
    }
}

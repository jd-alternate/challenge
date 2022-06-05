// see https://bheisler.github.io/criterion.rs/book/getting_started.html
use criterion::{criterion_group, criterion_main, Criterion};

use challenge::run_aux;

use rand::Rng;

fn generate_csv() -> String {
    let mut max_client_id = 0;
    let mut max_transaction_id = 0;

    let mut csv = String::from("type,client,tx,amount\n");
    let mut rng = rand::thread_rng();
    let mut client_transactions = Vec::<(u16, u32)>::new();

    for _ in 1..1000 {
        // need to decide on the type of event at random
        let value = rng.gen_range(0..10);
        match value {
            0..=7 => {
                let kind = if rng.gen_bool(0.5) {
                    "deposit"
                } else {
                    "withdrawal"
                };
                let client_id = if max_client_id == 0 || rng.gen_bool(0.1) {
                    max_client_id += 1;
                    max_client_id - 1
                } else {
                    rng.gen_range(0..=max_client_id)
                };
                max_transaction_id += 1;
                let transaction_id = max_transaction_id;
                let amount = rand::random::<u16>();
                client_transactions.push((client_id, transaction_id));
                csv.push_str(&format!(
                    "{},{},{},{}\n",
                    kind, client_id, transaction_id, amount
                ));
            }
            _ => {
                let kind = match rng.gen_range(0..2) {
                    0 => "dispute",
                    1 => "resolve",
                    2 => "chargeback",
                    _ => unreachable!(),
                };

                if let Some((client_id, transaction_id)) =
                    client_transactions.get(rng.gen_range(0..client_transactions.len()))
                {
                    csv.push_str(&format!("{},{},{},\n", kind, client_id, transaction_id));
                }
            }
        }
    }

    csv
}

fn criterion_benchmark(c: &mut Criterion) {
    // I'm going to create a huge CSV file and then run the program on it.
    let input = generate_csv();

    c.bench_function("large CSV", |b| {
        b.iter(|| {
            let mut output = Vec::new();
            crate::run_aux(&mut input.as_bytes(), &mut output).expect("Unexpected error");
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

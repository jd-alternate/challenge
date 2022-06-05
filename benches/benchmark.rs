use std::io;

// see https://bheisler.github.io/criterion.rs/book/getting_started.html
use criterion::{criterion_group, criterion_main, Criterion};
use rand::Rng;

use challenge::run_aux;

const CSV_ROW_COUNT: i32 = 100_000;
const SAMPLE_SIZE: usize = 20;

fn generate_csv() -> String {
    let mut max_client_id = 0;
    let mut max_transaction_id = 0;

    let mut csv = String::from("type,client,tx,amount\n");
    let mut rng = rand::thread_rng();
    let mut client_transactions = Vec::<(u16, u32)>::new();

    for _ in 1..CSV_ROW_COUNT {
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
                    rng.gen_range(0..=max_client_id - 1)
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

                if client_transactions.len() > 0 {
                    let (client_id, transaction_id) = client_transactions
                        .get(rng.gen_range(0..client_transactions.len()))
                        .expect("Expected a client transaction.");
                    csv.push_str(&format!("{},{},{},\n", kind, client_id, transaction_id));
                }
            }
        }
    }

    csv
}

fn criterion_benchmark(c: &mut Criterion) {
    let input = generate_csv();

    let mut group = c.benchmark_group("run_aux");
    group.sample_size(SAMPLE_SIZE);
    group.bench_function("run_aux", |b| {
        b.iter(|| {
            let mut output = Vec::new();
            crate::run_aux(&mut input.as_bytes(), &mut output, &mut io::sink())
                .expect("Unexpected error");
        })
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

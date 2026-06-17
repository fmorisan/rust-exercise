mod format;

use crate::format::transaction::{Transaction, TransactionRow};

fn main() {
    let mut reader = csv::Reader::from_path("test.csv").unwrap();
    let transactions: Vec<Transaction> = reader.deserialize::<TransactionRow>()
        .filter_map(|tx| tx.ok())
        .filter_map(|row| TryInto::<Transaction>::try_into(row).ok())
        .collect();

    eprintln!("{:?}", transactions);
}

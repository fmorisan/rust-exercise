mod format;
mod engine;

use std::{path::PathBuf};
use clap::Parser;

use crate::{
    engine::{state::AccountState, transaction::Transaction},
    format::transaction::{ParsedTransaction, TransactionRow}
};

#[derive(Parser)]
struct Args {
    path: PathBuf
}

fn main() {
    let args = Args::parse();
    let mut reader = csv::Reader::from_path(args.path).unwrap();
    let transactions: Vec<Transaction> = reader.deserialize::<TransactionRow>()
        .filter_map(|tx| tx.ok())
        .filter_map(|row| TryInto::<ParsedTransaction>::try_into(row).ok())
        .map(|tx| Transaction::from(tx))
        .collect();

    eprintln!("{:?}", transactions);

    let mut state = AccountState::new();

    for tx in transactions {
        // Discarding errors for now...
        let _ = state.apply_transaction(tx);
    }
}

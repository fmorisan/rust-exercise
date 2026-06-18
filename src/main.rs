mod format;
mod engine;

use std::{io::{LineWriter, stdout}, path::PathBuf};
use clap::Parser;

use crate::{
    engine::transaction::Transaction,
    engine::state::AccountState,
    format::account::Account as FormatAccount,
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

    let mut state = AccountState::new();

    for tx in transactions {
        // Discarding errors for now...
        let _ = state.apply_transaction(tx);
    }

    let mut writer = csv::Writer::from_writer(LineWriter::new(stdout()));

    for (uid, account) in state.all_accounts() {
        let record = FormatAccount::from_engine(*uid, account);
        let _ = writer.serialize(record);
    }
}

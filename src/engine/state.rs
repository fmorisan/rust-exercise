use std::collections::{BTreeMap, HashMap};

use crate::engine::{
    account::Account,
    transaction::Transaction
};

pub struct AccountState {
    /// Account registry, using BTreeMap for stable ordering
    accounts: BTreeMap<u16, Account>,
    /// Transaction registry
    ledger: HashMap<u32, Transaction>
}

#[derive(Debug)]
pub struct TransactionOperationError;

impl AccountState {
    pub fn new() -> Self {
        AccountState {
            accounts: BTreeMap::new(),
            ledger: HashMap::new()
        }
    }

    pub fn apply_transaction(&mut self, tx: Transaction) -> Result<(), TransactionOperationError> {
        todo!();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_works() {
        assert!(true);
    }
}

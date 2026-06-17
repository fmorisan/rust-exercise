use std::collections::{BTreeMap, HashMap};

use crate::{engine::{
    account::{Account, AccountOperationError},
    transaction::{Transaction, TransactionStateInvalid}
}, format::transaction::ParsedTransaction};

pub struct AccountState {
    /// Account registry, using BTreeMap for stable ordering
    accounts: BTreeMap<u16, Account>,
    /// Transaction registry
    ledger: HashMap<u32, Transaction>
}

#[derive(Debug)]
pub enum TransactionOperationError {
    InvalidTransaction,
    InvalidDispute,
    InvalidChargeback,
    TransactionError(TransactionStateInvalid),
    AccountError(AccountOperationError)
}

impl From<TransactionStateInvalid> for TransactionOperationError {
    fn from(value: TransactionStateInvalid) -> Self {
        Self::TransactionError(value)
    }
}

impl From<AccountOperationError> for TransactionOperationError {
    fn from(value: AccountOperationError) -> Self {
        Self::AccountError(value)
    }
}

impl AccountState {
    pub fn new() -> Self {
        AccountState {
            accounts: BTreeMap::new(),
            ledger: HashMap::new()
        }
    }

    pub fn get_account(&self, client: u16) -> Option<&Account> {
        self.accounts.get(&client)
    }

    fn get_or_insert_account(&mut self, client: u16) -> &mut Account {
        self.accounts.entry(client).or_insert(Account::default())
    }

    pub fn apply_transaction(&mut self, tx: Transaction) -> Result<(), TransactionOperationError> {
        match tx.transaction() {
            ParsedTransaction::Deposit { client, id, amount } => {
                let account = self.get_or_insert_account(*client);
                account.credit_amount(amount)?;
                self.ledger.insert(*id, tx);
                return Ok(());
            },
            ParsedTransaction::Withdrawal { client, id, amount } => {
                let account = self.get_or_insert_account(*client);
                account.debit_amount(amount)?;
                self.ledger.insert(*id, tx);
                return Ok(());
            },
            ParsedTransaction::Dispute { client, id } => {
                if let Some(disputed) = self.ledger.get_mut(id) {
                    match disputed.transaction() {
                        ParsedTransaction::Deposit { client: d_client, amount, .. } => {
                            if client != d_client {
                                return Err(TransactionOperationError::InvalidDispute);
                            } else {
                                let account = self.accounts.get_mut(client).unwrap();
                                account.hold_amount(amount)?;
                                disputed.dispute()?;
                            }
                        },
                        _ => {
                            return Err(TransactionOperationError::InvalidTransaction);
                        }

                    }
                }
            },
            ParsedTransaction::Resolve { client, id } => {
                if let Some(disputed) = self.ledger.get_mut(id) {
                    match disputed.transaction() {
                        ParsedTransaction::Deposit { client: d_client, amount, .. } => {
                            if client != d_client {
                                return Err(TransactionOperationError::InvalidDispute);
                            } else {
                                let account = self.accounts.get_mut(client).unwrap();
                                account.release_amount(amount)?;
                                disputed.resolve()?;
                            }
                        },
                        _ => {
                            return Err(TransactionOperationError::InvalidTransaction);
                        }

                    }
                }
            },
            ParsedTransaction::Chargeback { client, id } => {
                if let Some(disputed) = self.ledger.get_mut(id) {
                    match disputed.transaction() {
                        ParsedTransaction::Deposit { client: d_client, amount, .. } => {
                            if client != d_client {
                                return Err(TransactionOperationError::InvalidDispute);
                            } else {
                                let account = self.accounts.get_mut(client).unwrap();
                                account.release_amount(amount)?;
                                account.debit_amount(amount)?;
                                account.lock()?;
                                disputed.chargeback()?;
                            }
                        },
                        _ => {
                            return Err(TransactionOperationError::InvalidTransaction);
                        }

                    }
                }
            }
        }
        Ok(())
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

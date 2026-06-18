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
    DuplicateTransaction,
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

    pub fn get_transaction(&self, id: u32) -> Option<&Transaction> {
        self.ledger.get(&id)
    }

    fn get_or_insert_account(&mut self, client: u16) -> &mut Account {
        self.accounts.entry(client).or_insert(Account::default())
    }

    pub fn apply_transaction(&mut self, tx: Transaction) -> Result<(), TransactionOperationError> {
        match tx.transaction() {
            ParsedTransaction::Deposit { client, id, amount } => {
                if let Some(_) = self.ledger.get(id) {
                    return Err(TransactionOperationError::DuplicateTransaction);
                }
                let account = self.get_or_insert_account(*client);
                account.credit_amount(amount)?;
                self.ledger.insert(*id, tx);
                return Ok(());
            },
            ParsedTransaction::Withdrawal { client, id, amount } => {
                if let Some(_) = self.ledger.get(id) {
                    return Err(TransactionOperationError::DuplicateTransaction);
                }
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
    use rust_decimal::Decimal;

use crate::engine::transaction::TransactionState;

use super::*;

    #[test]
    fn it_works() {
        assert!(true);
    }

    #[test]
    fn process_deposit() {
        let tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(10) };
        let mut state = AccountState::new();

        let result = state.apply_transaction(tx.into());
        assert!(result.is_ok());

        let account = state.get_account(1).unwrap();
        assert_eq!(account.available(), Decimal::from(10));
    }

    #[test]
    fn process_withdrawal() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(10) };
        let withdraw_tx = ParsedTransaction::Withdrawal { client: 1, id: 2, amount: Decimal::from(10) };

        let mut state = AccountState::new();

        let result = state.apply_transaction(deposit_tx.into());
        assert!(result.is_ok());

        let available = state.get_account(1).unwrap().available();
        assert_eq!(available, Decimal::from(10));

        let result = state.apply_transaction(withdraw_tx.into());
        assert!(result.is_ok());

        let available = state.get_account(1).unwrap().available();
        assert_eq!(available, Decimal::from(0));
    }

    #[test]
    fn process_dispute() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(10) };
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 1 };

        let mut state = AccountState::new();

        let _ = state.apply_transaction(deposit_tx.into());
        let available = state.get_account(1).unwrap().available();
        assert_eq!(available, Decimal::from(10));

        assert!(matches!(
            state.apply_transaction(dispute_tx.into()),
            Ok(_)
        ));

        let available = state.get_account(1).unwrap().available();
        assert_eq!(available, Decimal::ZERO);

        let tx = state.get_transaction(1).unwrap();
        assert!(matches!(
            tx.state(),
            TransactionState::Disputed
        ));
    }

    #[test]
    fn process_resolve() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(10) };
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 1 };
        let resolve_tx = ParsedTransaction::Resolve { client: 1, id: 1 };

        let mut state = AccountState::new();

        let _ = state.apply_transaction(deposit_tx.into());
        let available = state.get_account(1).unwrap().available();
        assert_eq!(available, Decimal::from(10));

        assert!(matches!(
            state.apply_transaction(dispute_tx.into()),
            Ok(_)
        ));

        let available = state.get_account(1).unwrap().available();
        assert_eq!(available, Decimal::ZERO);

        let tx = state.get_transaction(1).unwrap();
        assert!(matches!(
            tx.state(),
            TransactionState::Disputed
        ));

        assert!(matches!(
            state.apply_transaction(resolve_tx.into()),
            Ok(_)
        ));

        let available = state.get_account(1).unwrap().available();
        assert_eq!(available, Decimal::from(10));

        let tx = state.get_transaction(1).unwrap();
        assert!(matches!(
            tx.state(),
            TransactionState::Normal
        ));
    }

    #[test]
    fn process_chargeback() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(10) };
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 1 };
        let chargeback_tx = ParsedTransaction::Chargeback { client: 1, id: 1 };

        let mut state = AccountState::new();

        let _ = state.apply_transaction(deposit_tx.into());
        let available = state.get_account(1).unwrap().available();
        assert_eq!(available, Decimal::from(10));

        assert!(matches!(
            state.apply_transaction(dispute_tx.into()),
            Ok(_)
        ));

        let available = state.get_account(1).unwrap().available();
        assert_eq!(available, Decimal::ZERO);

        let tx = state.get_transaction(1).unwrap();
        assert!(matches!(
            tx.state(),
            TransactionState::Disputed
        ));

        assert!(matches!(
            state.apply_transaction(chargeback_tx.into()),
            Ok(_)
        ));

        let account = state.get_account(1).unwrap();
        assert_eq!(account.available(), Decimal::ZERO);
        assert!(account.locked());

        let tx = state.get_transaction(1).unwrap();
        assert!(matches!(
            tx.state(),
            TransactionState::ChargedBack
        ));
    }
}

use std::collections::{BTreeMap, HashMap, btree_map::Iter};

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
    /// The transaction state change has failed
    TransactionError(TransactionStateInvalid),
    /// An error has ocurred during account state transition
    #[allow(dead_code)]
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

    pub fn all_accounts<'a>(&'a self) -> Iter<'a, u16, Account> {
        (&self.accounts).into_iter()
    }

    #[cfg(test)]
    pub fn get_account(&self, client: u16) -> Option<&Account> {
        self.accounts.get(&client)
    }

    #[cfg(test)]
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
                                let amt = amount.clone();
                                disputed.dispute()?;
                                let account = self.accounts.get_mut(client).unwrap();
                                account.hold_amount(&amt)?;
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
                                let amt = amount.clone();
                                disputed.resolve()?;
                                let account = self.accounts.get_mut(client).unwrap();
                                account.release_amount(&amt)?;
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
                                let amt = amount.clone();
                                disputed.chargeback()?;
                                let account = self.accounts.get_mut(client).unwrap();
                                account.release_amount(&amt)?;
                                account.debit_chargeback(&amt)?;
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
    use decimal_rs::Decimal;
    use std::assert_matches;

    use crate::engine::transaction::TransactionState;

    use super::*;

    // --- Adversarial: dispute/resolve/chargeback on wrong tx type ---

    #[test]
    fn dispute_withdrawal_returns_invalid_transaction() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let withdraw_tx = ParsedTransaction::Withdrawal { client: 1, id: 2, amount: Decimal::from(50) };
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 2 };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());
        let _ = state.apply_transaction(withdraw_tx.into());

        assert!(matches!(
            state.apply_transaction(dispute_tx.into()),
            Err(TransactionOperationError::InvalidTransaction)
        ));
    }

    #[test]
    fn resolve_withdrawal_returns_invalid_transaction() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let withdraw_tx = ParsedTransaction::Withdrawal { client: 1, id: 2, amount: Decimal::from(50) };
        let resolve_tx = ParsedTransaction::Resolve { client: 1, id: 2 };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());
        let _ = state.apply_transaction(withdraw_tx.into());

        assert!(matches!(
            state.apply_transaction(resolve_tx.into()),
            Err(TransactionOperationError::InvalidTransaction)
        ));
    }

    #[test]
    fn chargeback_withdrawal_returns_invalid_transaction() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let withdraw_tx = ParsedTransaction::Withdrawal { client: 1, id: 2, amount: Decimal::from(50) };
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 2 };
        let chargeback_tx = ParsedTransaction::Chargeback { client: 1, id: 2 };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());
        let _ = state.apply_transaction(withdraw_tx.into());

        let _ = state.apply_transaction(dispute_tx.into());
        assert!(matches!(
            state.apply_transaction(chargeback_tx.into()),
            Err(TransactionOperationError::InvalidTransaction)
        ));
    }

    // --- Adversarial: non-existent transaction IDs ---

    #[test]
    fn dispute_non_existent_tx_is_ignored() {
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 999 };
        let mut state = AccountState::new();

        let result = state.apply_transaction(dispute_tx.into());
        assert!(result.is_ok());
        assert!(state.get_account(1).is_none());
    }

    #[test]
    fn resolve_non_existent_tx_is_ignored() {
        let resolve_tx = ParsedTransaction::Resolve { client: 1, id: 999 };
        let mut state = AccountState::new();

        let result = state.apply_transaction(resolve_tx.into());
        assert!(result.is_ok());
    }

    #[test]
    fn chargeback_non_existent_tx_is_ignored() {
        let chargeback_tx = ParsedTransaction::Chargeback { client: 1, id: 999 };
        let mut state = AccountState::new();

        let result = state.apply_transaction(chargeback_tx.into());
        assert!(result.is_ok());
    }

    // --- Adversarial: cross-client attacks ---

    #[test]
    fn cross_client_dispute_returns_invalid_dispute() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let dispute_tx = ParsedTransaction::Dispute { client: 2, id: 1 };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());

        assert!(matches!(
            state.apply_transaction(dispute_tx.into()),
            Err(TransactionOperationError::InvalidDispute)
        ));

        let account = state.get_account(1).unwrap();
        assert_eq!(account.available(), Decimal::from(100));
        assert!(state.get_account(2).is_none());
    }

    #[test]
    fn cross_client_resolve_returns_invalid_dispute() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 1 };
        let resolve_tx = ParsedTransaction::Resolve { client: 2, id: 1 };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());
        let _ = state.apply_transaction(dispute_tx.into());

        assert!(matches!(
            state.apply_transaction(resolve_tx.into()),
            Err(TransactionOperationError::InvalidDispute)
        ));

        let account = state.get_account(1).unwrap();
        assert_eq!(account.available(), Decimal::ZERO);
    }

    #[test]
    fn cross_client_chargeback_returns_invalid_dispute() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 1 };
        let chargeback_tx = ParsedTransaction::Chargeback { client: 2, id: 1 };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());
        let _ = state.apply_transaction(dispute_tx.into());

        assert!(matches!(
            state.apply_transaction(chargeback_tx.into()),
            Err(TransactionOperationError::InvalidDispute)
        ));

        let account = state.get_account(1).unwrap();
        assert_eq!(account.available(), Decimal::ZERO);
        assert!(!account.locked());
    }

    // --- Adversarial: insufficient funds ---

    #[test]
    fn withdrawal_exceeding_available_fails() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(50) };
        let withdraw_tx = ParsedTransaction::Withdrawal { client: 1, id: 2, amount: Decimal::from(100) };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());

        assert!(matches!(
            state.apply_transaction(withdraw_tx.into()),
            Err(TransactionOperationError::AccountError(AccountOperationError::BalanceInsufficient))
        ));

        let account = state.get_account(1).unwrap();
        assert_eq!(account.available(), Decimal::from(50));
    }

    #[test]
    fn withdrawal_from_empty_account_fails() {
        let withdraw_tx = ParsedTransaction::Withdrawal { client: 1, id: 1, amount: Decimal::ONE };
        let mut state = AccountState::new();

        assert!(matches!(
            state.apply_transaction(withdraw_tx.into()),
            Err(TransactionOperationError::AccountError(AccountOperationError::BalanceInsufficient))
        ));

        assert!(state.get_account(1).is_some());
        assert_eq!(state.get_account(1).unwrap().available(), Decimal::ZERO);
    }

    #[test]
    fn withdrawal_limited_by_held_funds() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let withdraw_tx = ParsedTransaction::Withdrawal { client: 1, id: 2, amount: Decimal::from(80) };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());

        assert_eq!(state.get_account(1).unwrap().available(), Decimal::from(100));

        let _ = state.apply_transaction(withdraw_tx.into());
        let withdraw_tx2 = ParsedTransaction::Withdrawal { client: 1, id: 3, amount: Decimal::from(80) };
        assert!(matches!(
            state.apply_transaction(withdraw_tx2.into()),
            Err(TransactionOperationError::AccountError(AccountOperationError::BalanceInsufficient))
        ));
    }

    // --- Adversarial: operations on locked account ---

    #[test]
    fn deposit_to_locked_account_fails() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 1 };
        let chargeback_tx = ParsedTransaction::Chargeback { client: 1, id: 1 };
        let second_deposit = ParsedTransaction::Deposit { client: 1, id: 2, amount: Decimal::from(50) };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());
        let _ = state.apply_transaction(dispute_tx.into());
        let _ = state.apply_transaction(chargeback_tx.into());

        assert!(matches!(
            state.apply_transaction(second_deposit.into()),
            Err(TransactionOperationError::AccountError(AccountOperationError::AccountLocked))
        ));

        let account = state.get_account(1).unwrap();
        assert_eq!(account.available(), Decimal::ZERO);
        assert!(account.locked());
    }

    #[test]
    fn withdrawal_from_locked_account_fails() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 1 };
        let chargeback_tx = ParsedTransaction::Chargeback { client: 1, id: 1 };
        let withdraw_tx = ParsedTransaction::Withdrawal { client: 1, id: 2, amount: Decimal::from(50) };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());
        let _ = state.apply_transaction(dispute_tx.into());
        let _ = state.apply_transaction(chargeback_tx.into());

        assert!(matches!(
            state.apply_transaction(withdraw_tx.into()),
            Err(TransactionOperationError::AccountError(AccountOperationError::AccountLocked))
        ));
    }

    #[test]
    fn dispute_on_locked_account_fails() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let deposit_tx2 = ParsedTransaction::Deposit { client: 1, id: 2, amount: Decimal::from(50) };
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 1 };
        let chargeback_tx = ParsedTransaction::Chargeback { client: 1, id: 1 };
        let dispute_tx2 = ParsedTransaction::Dispute { client: 1, id: 2 };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());
        let _ = state.apply_transaction(deposit_tx2.into());
        let _ = state.apply_transaction(dispute_tx.into());
        let _ = state.apply_transaction(chargeback_tx.into());

        assert!(matches!(
            state.apply_transaction(dispute_tx2.into()),
            Err(TransactionOperationError::AccountError(AccountOperationError::AccountLocked))
        ));
    }

    #[test]
    fn resolve_on_locked_account_fails() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let deposit_tx2 = ParsedTransaction::Deposit { client: 1, id: 2, amount: Decimal::from(50) };
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 1 };
        let chargeback_tx = ParsedTransaction::Chargeback { client: 1, id: 1 };
        let resolve_tx = ParsedTransaction::Resolve { client: 1, id: 2 };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());
        let _ = state.apply_transaction(deposit_tx2.into());
        let _ = state.apply_transaction(dispute_tx.into());
        let _ = state.apply_transaction(chargeback_tx.into());

        assert!(state.get_account(1).unwrap().locked());

        assert!(matches!(
            state.apply_transaction(resolve_tx.into()),
            Err(TransactionOperationError::TransactionError(_))
        ));
    }

    // --- Adversarial: invalid state transitions ---

    #[test]
    fn double_dispute_fails() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 1 };
        let dispute_tx2 = ParsedTransaction::Dispute { client: 1, id: 1 };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());
        let _ = state.apply_transaction(dispute_tx.into());

        assert_eq!(state.get_account(1).unwrap().available(), Decimal::ZERO);

        assert!(matches!(
            state.apply_transaction(dispute_tx2.into()),
            Err(TransactionOperationError::TransactionError(_))
        ));
    }

    #[test]
    fn resolve_without_prior_dispute_fails() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let resolve_tx = ParsedTransaction::Resolve { client: 1, id: 1 };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());

        assert!(matches!(
            state.apply_transaction(resolve_tx.into()),
            Err(TransactionOperationError::TransactionError(_))
        ));
    }

    #[test]
    fn chargeback_without_prior_dispute_fails() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let chargeback_tx = ParsedTransaction::Chargeback { client: 1, id: 1 };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());

        assert!(matches!(
            state.apply_transaction(chargeback_tx.into()),
            Err(TransactionOperationError::TransactionError(_))
        ));
    }

    #[test]
    fn chargeback_after_resolve_fails() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 1 };
        let resolve_tx = ParsedTransaction::Resolve { client: 1, id: 1 };
        let chargeback_tx = ParsedTransaction::Chargeback { client: 1, id: 1 };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());
        let _ = state.apply_transaction(dispute_tx.into());
        let _ = state.apply_transaction(resolve_tx.into());

        assert!(!state.get_account(1).unwrap().locked());

        assert!(matches!(
            state.apply_transaction(chargeback_tx.into()),
            Err(TransactionOperationError::TransactionError(_))
        ));

        assert!(!state.get_account(1).unwrap().locked());
    }

    #[test]
    fn dispute_already_charged_back_tx_fails() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let deposit_tx2 = ParsedTransaction::Deposit { client: 1, id: 2, amount: Decimal::from(50) };
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 1 };
        let chargeback_tx = ParsedTransaction::Chargeback { client: 1, id: 1 };
        let dispute_tx2 = ParsedTransaction::Dispute { client: 1, id: 1 };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());
        let _ = state.apply_transaction(deposit_tx2.into());
        let _ = state.apply_transaction(dispute_tx.into());
        let _ = state.apply_transaction(chargeback_tx.into());

        assert!(matches!(
            state.apply_transaction(dispute_tx2.into()),
            Err(TransactionOperationError::TransactionError(_))
        ));
    }

    #[test]
    fn resolve_already_charged_back_tx_fails() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 1 };
        let chargeback_tx = ParsedTransaction::Chargeback { client: 1, id: 1 };
        let resolve_tx = ParsedTransaction::Resolve { client: 1, id: 1 };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());
        let _ = state.apply_transaction(dispute_tx.into());
        let _ = state.apply_transaction(chargeback_tx.into());

        assert!(matches!(
            state.apply_transaction(resolve_tx.into()),
            Err(TransactionOperationError::TransactionError(_))
        ));
    }

    // --- Adversarial: partial mutation bugs ---

    #[test]
    fn resolve_wrong_tx_after_dispute_should_not_release_held() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let deposit_tx2 = ParsedTransaction::Deposit { client: 1, id: 2, amount: Decimal::from(100) };
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 1 };
        let resolve_tx = ParsedTransaction::Resolve { client: 1, id: 2 };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());
        let _ = state.apply_transaction(deposit_tx2.into());

        let _ = state.apply_transaction(dispute_tx.into());
        let account = state.get_account(1).unwrap();
        assert_eq!(account.available(), Decimal::from(100));

        let result = state.apply_transaction(resolve_tx.into());

        if result.is_ok() {
            panic!("resolve on a non-disputed tx should fail");
        }

        let account = state.get_account(1).unwrap();
        assert_eq!(
            account.available(),
            Decimal::from(100),
            "held should not have been released for a tx that was not disputed"
        );
    }

    // --- Adversarial: duplicate transaction IDs ---

    #[test]
    fn duplicate_deposit_tx_ids_should_not_cause_double_credit() {
        let deposit_tx1 = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let deposit_tx2 = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(50) };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx1.into());
        assert!(matches!(
            state.apply_transaction(deposit_tx2.into()),
            Err(TransactionOperationError::DuplicateTransaction)
        ));

        let account = state.get_account(1).unwrap();
        assert_eq!(account.available(), Decimal::from(100));
    }

    // --- Adversarial: multi-client isolation ---

    #[test]
    fn multiple_clients_isolated() {
        let d1 = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let d2 = ParsedTransaction::Deposit { client: 2, id: 2, amount: Decimal::from(200) };
        let w1 = ParsedTransaction::Withdrawal { client: 1, id: 3, amount: Decimal::from(50) };
        let dispute2 = ParsedTransaction::Dispute { client: 2, id: 2 };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(d1.into());
        let _ = state.apply_transaction(d2.into());
        let _ = state.apply_transaction(w1.into());
        let _ = state.apply_transaction(dispute2.into());

        let a1 = state.get_account(1).unwrap();
        assert_eq!(a1.available(), Decimal::from(50));

        let a2 = state.get_account(2).unwrap();
        assert_eq!(a2.available(), Decimal::ZERO);
    }

    #[test]
    fn chargeback_one_client_does_not_lock_another() {
        let d1 = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let d2 = ParsedTransaction::Deposit { client: 2, id: 2, amount: Decimal::from(100) };
        let dispute1 = ParsedTransaction::Dispute { client: 1, id: 1 };
        let chargeback1 = ParsedTransaction::Chargeback { client: 1, id: 1 };
        let w2 = ParsedTransaction::Withdrawal { client: 2, id: 3, amount: Decimal::from(50) };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(d1.into());
        let _ = state.apply_transaction(d2.into());
        let _ = state.apply_transaction(dispute1.into());
        let _ = state.apply_transaction(chargeback1.into());
        let _ = state.apply_transaction(w2.into());

        assert!(state.get_account(1).unwrap().locked());
        assert!(!state.get_account(2).unwrap().locked());
        assert_eq!(state.get_account(2).unwrap().available(), Decimal::from(50));
    }

    // --- Adversarial: re-dispute after resolve ---

    #[test]
    fn dispute_resolve_dispute_cycle() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 1 };
        let resolve_tx = ParsedTransaction::Resolve { client: 1, id: 1 };
        let dispute_tx2 = ParsedTransaction::Dispute { client: 1, id: 1 };
        let resolve_tx2 = ParsedTransaction::Resolve { client: 1, id: 1 };
        let _resolve_tx2 = resolve_tx2;

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());

        let _ = state.apply_transaction(dispute_tx.into());
        assert_eq!(state.get_account(1).unwrap().available(), Decimal::ZERO);

        let _ = state.apply_transaction(resolve_tx.into());
        assert_eq!(state.get_account(1).unwrap().available(), Decimal::from(100));

        let _ = state.apply_transaction(dispute_tx2.into());
        assert_eq!(state.get_account(1).unwrap().available(), Decimal::ZERO);

        let tx = state.get_transaction(1).unwrap();
        assert!(matches!(tx.state(), TransactionState::Disputed));
    }

    // --- Adversarial: dispute then partial withdrawal then chargeback ---

    #[test]
    fn dispute_withdraw_then_chargeback() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(200) };
        let deposit_tx2 = ParsedTransaction::Deposit { client: 1, id: 2, amount: Decimal::from(100) };
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 2 };
        let withdraw_tx = ParsedTransaction::Withdrawal { client: 1, id: 3, amount: Decimal::from(50) };
        let chargeback_tx = ParsedTransaction::Chargeback { client: 1, id: 2 };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());
        let _ = state.apply_transaction(deposit_tx2.into());

        let _ = state.apply_transaction(dispute_tx.into());
        assert_eq!(state.get_account(1).unwrap().available(), Decimal::from(200));
        assert!(!state.get_account(1).unwrap().locked());

        let _ = state.apply_transaction(withdraw_tx.into());
        assert_eq!(state.get_account(1).unwrap().available(), Decimal::from(150));

        let _ = state.apply_transaction(chargeback_tx.into());

        let account = state.get_account(1).unwrap();
        assert_eq!(account.available(), Decimal::from(150));
        assert!(account.locked());
        assert!(matches!(state.get_transaction(2).unwrap().state(), TransactionState::ChargedBack));
    }

    // --- Adversarial: multiple concurrent disputes on same account ---

    #[test]
    fn multiple_disputes_on_same_account() {
        let d1 = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let d2 = ParsedTransaction::Deposit { client: 1, id: 2, amount: Decimal::from(100) };
        let d3 = ParsedTransaction::Deposit { client: 1, id: 3, amount: Decimal::from(100) };
        let disp1 = ParsedTransaction::Dispute { client: 1, id: 1 };
        let disp2 = ParsedTransaction::Dispute { client: 1, id: 2 };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(d1.into());
        let _ = state.apply_transaction(d2.into());
        let _ = state.apply_transaction(d3.into());

        let _ = state.apply_transaction(disp1.into());
        assert_eq!(state.get_account(1).unwrap().available(), Decimal::from(200));

        let _ = state.apply_transaction(disp2.into());
        assert_eq!(state.get_account(1).unwrap().available(), Decimal::from(100));
    }

    // --- Adversarial: zero amount ---

    #[test]
    fn zero_deposit_succeeds() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::ZERO };
        let mut state = AccountState::new();

        let result = state.apply_transaction(deposit_tx.into());
        assert!(result.is_ok());

        let account = state.get_account(1).unwrap();
        assert_eq!(account.available(), Decimal::ZERO);
    }

    #[test]
    fn zero_withdrawal_succeeds() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(100) };
        let withdraw_tx = ParsedTransaction::Withdrawal { client: 1, id: 2, amount: Decimal::ZERO };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(deposit_tx.into());
        let _ = state.apply_transaction(withdraw_tx.into());

        assert_eq!(state.get_account(1).unwrap().available(), Decimal::from(100));
    }

    // --- Adversarial: account created on deposit does not exist before ---

    #[test]
    fn account_created_implicitly_on_deposit() {
        let deposit_tx = ParsedTransaction::Deposit { client: 42, id: 1, amount: Decimal::from(100) };
        let mut state = AccountState::new();

        assert!(state.get_account(42).is_none());
        let _ = state.apply_transaction(deposit_tx.into());
        assert!(state.get_account(42).is_some());
        assert_eq!(state.get_account(42).unwrap().available(), Decimal::from(100));
    }

    #[test]
    fn account_not_created_for_dispute_of_non_existent_client() {
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 999 };
        let mut state = AccountState::new();

        let _ = state.apply_transaction(dispute_tx.into());
        assert!(state.get_account(1).is_none());
    }

    // --- Adversarial: out of order tx IDs ---

    #[test]
    fn out_of_order_tx_ids() {
        let d1 = ParsedTransaction::Deposit { client: 1, id: 5, amount: Decimal::from(100) };
        let d2 = ParsedTransaction::Deposit { client: 1, id: 2, amount: Decimal::from(50) };
        let d3 = ParsedTransaction::Deposit { client: 1, id: 8, amount: Decimal::from(25) };

        let mut state = AccountState::new();
        let _ = state.apply_transaction(d1.into());
        let _ = state.apply_transaction(d2.into());
        let _ = state.apply_transaction(d3.into());

        let account = state.get_account(1).unwrap();
        assert_eq!(account.available(), Decimal::from(175));
        assert!(state.get_transaction(5).is_some());
        assert!(state.get_transaction(2).is_some());
        assert!(state.get_transaction(8).is_some());
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

    #[test]
    fn chargeback_after_withdraw_results_in_negative_balance() {
        let deposit_tx = ParsedTransaction::Deposit { client: 1, id: 1, amount: Decimal::from(10) };
        let withdraw_tx = ParsedTransaction::Withdrawal { client: 1, id: 2, amount: Decimal::from(10) };
        let dispute_tx = ParsedTransaction::Dispute { client: 1, id: 1 };
        let chargeback_tx = ParsedTransaction::Chargeback { client: 1, id: 1 };

        let mut state = AccountState::new();

        let _ = state.apply_transaction(deposit_tx.into());
        let available = state.get_account(1).unwrap().available();
        assert_eq!(available, Decimal::from(10));

        assert!(matches!(
            state.apply_transaction(withdraw_tx.into()),
            Ok(_)
        ));

        let available = state.get_account(1).unwrap().available();
        assert_eq!(available, Decimal::ZERO);

        assert_matches!(
            state.apply_transaction(dispute_tx.into()),
            Ok(_)
        );

        let tx = state.get_transaction(1).unwrap();
        assert!(matches!(
            tx.state(),
            TransactionState::Disputed
        ));

        assert_matches!(
            state.apply_transaction(chargeback_tx.into()),
            Ok(_)
        );

        let account = state.get_account(1).unwrap();
        assert_eq!(account.available(), Decimal::ZERO);
        assert!(account.locked());
        assert_eq!(account.held(), Decimal::from(0));
        assert_eq!(account.total(), Decimal::from(-10));

        let tx = state.get_transaction(1).unwrap();
        assert_matches!(
            tx.state(),
            TransactionState::ChargedBack
        );
    }
}

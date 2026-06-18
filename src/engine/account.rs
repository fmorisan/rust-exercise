use decimal_rs::Decimal;

#[derive(Default, Debug)]
pub struct Account {
    /// Total funds held in the account
    total: Decimal,
    /// Funds unavailable to the account due to disputes
    held: Decimal,
    /// Whether this account is locked and unable to receive any more transactions
    locked: bool,
}

#[derive(Debug, PartialEq)]
pub enum AccountOperationError {
    AccountLocked,
    BalanceInsufficient,
}

impl Account {
    /// Funds readily available to the account
    pub fn available(&self) -> Decimal {
        self.total.checked_sub(self.held).unwrap().max(Decimal::ZERO)
    }

    pub fn total(&self) -> Decimal {
        self.total
    }

    pub fn held(&self) -> Decimal {
        self.held
    }

    pub fn locked(&self) -> bool {
        self.locked
    }

    /// Credit an amount of money to the account.
    /// Cannot be called on locked accounts.
    pub fn credit_amount(&mut self, amount: &Decimal) -> Result<(), AccountOperationError> {
        if self.locked {
            Err(AccountOperationError::AccountLocked)
        } else {
            self.total = self.total.checked_add(*amount).unwrap();
            Ok(())
        }
    }

    /// Debit an amount from an account
    /// Fails on a locked account, or unavailable funds.
    pub fn debit_amount(&mut self, amount: &Decimal) -> Result<(), AccountOperationError> {
        if self.locked {
            Err(AccountOperationError::AccountLocked)
        } else if amount.gt(&self.available()) {
            Err(AccountOperationError::BalanceInsufficient)
        } else {
            self.total = self.total.checked_sub(*amount).unwrap();
            Ok(())
        }
    }

    /// Debit an amount and lock an account during a chargeback
    /// Fails on a locked account. MAY cause `total` to go negative.
    pub fn debit_chargeback(&mut self, amount: &Decimal) -> Result<(), AccountOperationError> {
        self.lock()?;
        self.total = self.total.checked_sub(*amount).unwrap();
        Ok(())
    }
    
    /// Stakes a hold on an account's funds (used when a transaction is disputed)
    /// Fails on locked accounts.
    pub fn hold_amount(&mut self, amount: &Decimal) -> Result<(), AccountOperationError> {
        if self.locked {
            Err(AccountOperationError::AccountLocked)
        } else {
            self.held = self.held.checked_add(*amount).unwrap();
            Ok(())
        }
    }

    /// Release a held amount (dispute was resolved)
    /// Fails on locked accounts. `amount` must be <= `account.held`.
    pub fn release_amount(&mut self, amount: &Decimal) -> Result<(), AccountOperationError> {
        if self.locked {
            Err(AccountOperationError::AccountLocked)
        } else if amount.gt(&self.held) {
            Err(AccountOperationError::BalanceInsufficient)
        } else {
            self.held = self.held.checked_sub(*amount).unwrap();
            Ok(())
        }
    }

    /// Locks an account for having charged back a transaction.
    pub fn lock(&mut self) -> Result<(), AccountOperationError>{
        if self.locked {
            Err(AccountOperationError::AccountLocked)
        } else {
            self.locked = true;
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn default_account() -> Account {
        Account::default()
    }

    // --- available() ---

    #[test]
    fn available_equals_total_minus_held() {
        let mut account = default_account();
        account.total = Decimal::from(100);
        account.held = Decimal::from(30);
        assert_eq!(account.available(), Decimal::from(70));
    }

    #[test]
    fn available_on_new_account_is_zero() {
        let account = default_account();
        assert_eq!(account.available(), Decimal::ZERO);
    }

    // --- credit_amount ---

    #[test]
    fn credit_increases_total() {
        let mut account = default_account();
        account.credit_amount(&Decimal::from(100)).unwrap();
        assert_eq!(account.total, Decimal::from(100));
        assert_eq!(account.available(), Decimal::from(100));
        assert_eq!(account.held, Decimal::ZERO);
    }

    #[test]
    fn credit_on_locked_account_fails() {
        let mut account = default_account();
        account.locked = true;
        assert_eq!(
            account.credit_amount(&Decimal::from(100)),
            Err(AccountOperationError::AccountLocked)
        );
        assert_eq!(account.total, Decimal::ZERO);
    }

    #[test]
    fn credit_accumulates_multiple_deposits() {
        let mut account = default_account();
        account.credit_amount(&Decimal::from(50)).unwrap();
        account.credit_amount(&Decimal::from(30)).unwrap();
        account.credit_amount(&Decimal::from(20)).unwrap();
        assert_eq!(account.total, Decimal::from(100));
    }

    // --- debit_amount ---

    #[test]
    fn debit_decreases_total() {
        let mut account = default_account();
        account.total = Decimal::from(100);
        account.debit_amount(&Decimal::from(30)).unwrap();
        assert_eq!(account.total, Decimal::from(70));
        assert_eq!(account.available(), Decimal::from(70));
    }

    #[test]
    fn debit_with_insufficient_funds_fails() {
        let mut account = default_account();
        account.total = Decimal::from(50);
        assert_eq!(
            account.debit_amount(&Decimal::from(100)),
            Err(AccountOperationError::BalanceInsufficient)
        );
        assert_eq!(account.total, Decimal::from(50));
    }

    #[test]
    fn debit_exact_available_succeeds() {
        let mut account = default_account();
        account.total = Decimal::from(100);
        account.debit_amount(&Decimal::from(100)).unwrap();
        assert_eq!(account.total, Decimal::ZERO);
        assert_eq!(account.available(), Decimal::ZERO);
    }

    #[test]
    fn debit_on_locked_account_fails() {
        let mut account = default_account();
        account.total = Decimal::from(100);
        account.locked = true;
        assert_eq!(
            account.debit_amount(&Decimal::from(30)),
            Err(AccountOperationError::AccountLocked)
        );
        assert_eq!(account.total, Decimal::from(100));
    }

    #[test]
    fn debit_considers_held_funds_unavailable() {
        let mut account = default_account();
        account.total = Decimal::from(100);
        account.held = Decimal::from(40);
        assert_eq!(account.available(), Decimal::from(60));
        assert_eq!(
            account.debit_amount(&Decimal::from(70)),
            Err(AccountOperationError::BalanceInsufficient)
        );
        account.debit_amount(&Decimal::from(60)).unwrap();
        assert_eq!(account.total, Decimal::from(40));
        assert_eq!(account.held, Decimal::from(40));
        assert_eq!(account.available(), Decimal::ZERO);
    }

    #[test]
    fn debit_from_zero_account_fails() {
        let account = default_account();
        assert_eq!(
            account.total, Decimal::ZERO
        );
        let mut account = default_account();
        assert_eq!(
            account.debit_amount(&Decimal::ONE),
            Err(AccountOperationError::BalanceInsufficient)
        );
    }

    // --- hold_amount ---

    #[test]
    fn hold_increases_held_total_unchanged() {
        let mut account = default_account();
        account.total = Decimal::from(100);
        account.hold_amount(&Decimal::from(30)).unwrap();
        assert_eq!(account.held, Decimal::from(30));
        assert_eq!(account.available(), Decimal::from(70));
        assert_eq!(account.total, Decimal::from(100));
    }

    #[test]
    fn hold_with_insufficient_available_succeeds() {
        let mut account = default_account();
        account.total = Decimal::from(50);
        account.held = Decimal::from(30);
        assert_eq!(account.available(), Decimal::from(20));
        assert_eq!(
            account.hold_amount(&Decimal::from(25)),
            Ok(())
        );
        assert_eq!(account.held, Decimal::from(55));
        assert_eq!(account.available(), Decimal::ZERO);
    }


    #[test]
    fn hold_on_locked_account_fails() {
        let mut account = default_account();
        account.total = Decimal::from(100);
        account.locked = true;
        assert_eq!(
            account.hold_amount(&Decimal::from(30)),
            Err(AccountOperationError::AccountLocked)
        );
        assert_eq!(account.held, Decimal::ZERO);
    }

    #[test]
    fn hold_accumulates_across_multiple_holds() {
        let mut account = default_account();
        account.total = Decimal::from(100);
        account.hold_amount(&Decimal::from(20)).unwrap();
        account.hold_amount(&Decimal::from(30)).unwrap();
        assert_eq!(account.held, Decimal::from(50));
        assert_eq!(account.available(), Decimal::from(50));
        assert_eq!(account.total, Decimal::from(100));
    }

    // --- release_amount ---

    #[test]
    fn release_decreases_held_total_unchanged() {
        let mut account = default_account();
        account.total = Decimal::from(100);
        account.held = Decimal::from(30);
        account.release_amount(&Decimal::from(10)).unwrap();
        assert_eq!(account.held, Decimal::from(20));
        assert_eq!(account.available(), Decimal::from(80));
        assert_eq!(account.total, Decimal::from(100));
    }

    #[test]
    fn release_more_than_held_fails() {
        let mut account = default_account();
        account.total = Decimal::from(100);
        account.held = Decimal::from(20);
        assert_eq!(
            account.release_amount(&Decimal::from(30)),
            Err(AccountOperationError::BalanceInsufficient)
        );
        assert_eq!(account.held, Decimal::from(20));
    }

    #[test]
    fn release_on_locked_account_fails() {
        let mut account = default_account();
        account.total = Decimal::from(100);
        account.held = Decimal::from(30);
        account.locked = true;
        assert_eq!(
            account.release_amount(&Decimal::from(10)),
            Err(AccountOperationError::AccountLocked)
        );
        assert_eq!(account.held, Decimal::from(30));
    }

    #[test]
    fn release_all_held_returns_all_to_available() {
        let mut account = default_account();
        account.total = Decimal::from(100);
        account.held = Decimal::from(100);
        account.release_amount(&Decimal::from(100)).unwrap();
        assert_eq!(account.held, Decimal::ZERO);
        assert_eq!(account.available(), Decimal::from(100));
    }

    #[test]
    fn release_on_account_with_no_held_fails() {
        let mut account = default_account();
        account.total = Decimal::from(100);
        assert_eq!(
            account.release_amount(&Decimal::ONE),
            Err(AccountOperationError::BalanceInsufficient)
        );
        assert_eq!(account.held, Decimal::ZERO);
    }

    // --- integration: full deposit/withdrawal/dispute/resolve lifecycle ---

    #[test]
    fn deposit_withdrawal_dispute_resolve_lifecycle() {
        let mut account = default_account();

        account.credit_amount(&Decimal::from(100)).unwrap();
        assert_eq!(account.total, Decimal::from(100));
        assert_eq!(account.available(), Decimal::from(100));
        assert_eq!(account.held, Decimal::ZERO);

        account.debit_amount(&Decimal::from(30)).unwrap();
        assert_eq!(account.total, Decimal::from(70));
        assert_eq!(account.available(), Decimal::from(70));

        account.hold_amount(&Decimal::from(50)).unwrap();
        assert_eq!(account.total, Decimal::from(70));
        assert_eq!(account.held, Decimal::from(50));
        assert_eq!(account.available(), Decimal::from(20));

        account.release_amount(&Decimal::from(50)).unwrap();
        assert_eq!(account.total, Decimal::from(70));
        assert_eq!(account.held, Decimal::ZERO);
        assert_eq!(account.available(), Decimal::from(70));
    }

    #[test]
    fn deposit_withdrawal_dispute_chargeback_lifecycle() {
        let mut account = default_account();

        account.credit_amount(&Decimal::from(100)).unwrap();
        account.debit_amount(&Decimal::from(30)).unwrap();
        assert_eq!(account.total, Decimal::from(70));
        assert_eq!(account.available(), Decimal::from(70));

        account.hold_amount(&Decimal::from(50)).unwrap();
        assert_eq!(account.total, Decimal::from(70));
        assert_eq!(account.held, Decimal::from(50));
        assert_eq!(account.available(), Decimal::from(20));

        account.debit_amount(&Decimal::from(20)).unwrap();
        assert_eq!(account.total, Decimal::from(50));
        assert_eq!(account.held, Decimal::from(50));
        assert_eq!(account.available(), Decimal::from(0));

        let res = account.lock();
        assert!(res.is_ok());
        assert!(account.locked);

        assert_eq!(
            account.credit_amount(&Decimal::from(10)),
            Err(AccountOperationError::AccountLocked)
        );
        assert_eq!(
            account.debit_amount(&Decimal::from(10)),
            Err(AccountOperationError::AccountLocked)
        );
        assert_eq!(
            account.hold_amount(&Decimal::from(10)),
            Err(AccountOperationError::AccountLocked)
        );
        assert_eq!(
            account.release_amount(&Decimal::from(10)),
            Err(AccountOperationError::AccountLocked)
        );
    }

    #[test]
    fn locked_account_rejects_all_operations() {
        let mut account = default_account();
        account.total = Decimal::from(100);
        account.held = Decimal::from(20);
        account.locked = true;

        assert!(matches!(
            account.credit_amount(&Decimal::from(10)),
            Err(AccountOperationError::AccountLocked)
        ));
        assert!(matches!(
            account.debit_amount(&Decimal::from(10)),
            Err(AccountOperationError::AccountLocked)
        ));
        assert!(matches!(
            account.hold_amount(&Decimal::from(10)),
            Err(AccountOperationError::AccountLocked)
        ));
        assert!(matches!(
            account.release_amount(&Decimal::from(10)),
            Err(AccountOperationError::AccountLocked)
        ));
        assert_eq!(account.total, Decimal::from(100));
        assert_eq!(account.held, Decimal::from(20));
    }

    #[test]
    fn account_cannot_be_locked_twice() {
        let mut account = default_account();

        let res = account.lock();
        assert!(res.is_ok());

        assert!(matches!(
            account.lock(),
            Err(AccountOperationError::AccountLocked)
        ));
    }

    #[test]
    fn chargeback_after_withdraw_results_in_negative_balance() {
        let mut account = default_account();

        assert!(matches!(
            account.credit_amount(&Decimal::from(10)),
            Ok(_)
        ));
        assert!(matches!(
            account.debit_amount(&Decimal::from(10)),
            Ok(_)
        ));
        assert!(matches!(
            account.hold_amount(&Decimal::from(10)),
            Ok(_)
        ));

        assert!(matches!(
            account.debit_chargeback(&Decimal::from(10)),
            Ok(_)
        ));

        assert_eq!(account.held, Decimal::from(10));
        assert_eq!(account.available(), Decimal::ZERO);
        assert_eq!(account.total, Decimal::from(-10));
        assert!(account.locked);
    }
}

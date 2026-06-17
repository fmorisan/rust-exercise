use rust_decimal::Decimal;

#[derive(Default, Debug)]
pub struct Account {
    /// Total funds held in the account
    total: Decimal,
    /// Funds unavailable to the account due to disputes
    held: Decimal,
    /// Whether this account is locked and unable to receive any more transactions
    locked: bool,
}

pub enum AccountOperationError {
    AccountLocked,
    BalanceInsufficient,
    InvariantViolation
}

impl Account {
    /// Funds readily available to the account
    pub fn available(&self) -> Decimal {
        self.total.checked_sub(self.held).unwrap()
    }

    pub fn credit_amount(&mut self, amount: Decimal) -> Result<(), AccountOperationError> {
        if self.locked {
            Err(AccountOperationError::AccountLocked)
        } else {
            self.total = self.total.checked_add(amount).unwrap();
            Ok(())
        }
    }

    pub fn debit_amount(&mut self, amount: Decimal) -> Result<(), AccountOperationError> {
        if self.locked {
            Err(AccountOperationError::AccountLocked)
        } else if amount.gt(&self.available()) {
            Err(AccountOperationError::BalanceInsufficient)
        } else {
            self.total = self.total.checked_sub(amount).unwrap();
            Ok(())
        }
    }
    
    pub fn hold_amount(&mut self, amount: Decimal) -> Result<(), AccountOperationError> {
        if self.locked {
            Err(AccountOperationError::AccountLocked)
        } else if amount.gt(&self.available()) {
            Err(AccountOperationError::BalanceInsufficient)
        } else {
            self.held = self.held.checked_sub(amount).unwrap();
            Ok(())
        }
    }

    pub fn release_amount(&mut self, amount: Decimal) -> Result<(), AccountOperationError> {
        if self.locked {
            Err(AccountOperationError::AccountLocked)
        } else if amount.gt(&self.held) {
            Err(AccountOperationError::BalanceInsufficient)
        } else {
            self.held = self.held.checked_sub(amount).unwrap();
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
}

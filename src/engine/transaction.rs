use crate::format::transaction::ParsedTransaction;

#[derive(Debug)]
pub enum TransactionState {
    /// The default state for transactions that arrive to the system
    Normal,
    /// Transactions may be disputed, in which case the transaction's amount must be held
    Disputed,
    /// Transactions may be charged back, in which case the underlying transaction is reverted and
    /// the account is locked.
    ChargedBack
}

#[derive(Debug)]
pub struct Transaction {
    tx: ParsedTransaction,
    state: TransactionState
}

impl From<ParsedTransaction> for Transaction {
    fn from(value: ParsedTransaction) -> Self {
        Self {
            state: TransactionState::Normal,
            tx: value
        }
    }
}

impl Transaction {
    fn valid_state_transition(&self, new_state: TransactionState) -> bool {
        matches!(
            (&self.state, new_state),
            // Transactions may be disputed
            (TransactionState::Normal, TransactionState::Disputed)
            // Disputed transactions may be resolved (back to normal state) or charged back
            | (TransactionState::Disputed, TransactionState::Normal | TransactionState::ChargedBack)
            // Charged back transactions are not disputable again.
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
}

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

#[derive(Debug)]
pub struct TransactionStateInvalid;

impl Transaction {
    pub fn transaction(&self) -> &ParsedTransaction {
        &self.tx
    }

    #[cfg(test)]
    pub fn state(&self) -> &TransactionState {
        &self.state
    }

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

    pub fn dispute(&mut self) -> Result<(), TransactionStateInvalid> {
        if self.valid_state_transition(TransactionState::Disputed) {
            self.state = TransactionState::Disputed;
            Ok(())
        } else {
            Err(TransactionStateInvalid)
        }
    }

    pub fn resolve(&mut self) -> Result<(), TransactionStateInvalid> {
        if self.valid_state_transition(TransactionState::Normal) {
            self.state = TransactionState::Normal;
            Ok(())
        } else {
            Err(TransactionStateInvalid)
        }
    }

    pub fn chargeback(&mut self) -> Result<(), TransactionStateInvalid> {
        if self.valid_state_transition(TransactionState::ChargedBack) {
            self.state = TransactionState::ChargedBack;
            Ok(())
        } else {
            Err(TransactionStateInvalid)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::format::transaction::ParsedTransaction;
    use decimal_rs::Decimal;

    fn make_deposit_tx(client: u16, id: u32, amount: Decimal) -> Transaction {
        Transaction::from(ParsedTransaction::Deposit { client, id, amount })
    }

    fn make_withdrawal_tx(client: u16, id: u32, amount: Decimal) -> Transaction {
        Transaction::from(ParsedTransaction::Withdrawal { client, id, amount })
    }

    // --- From<ParsedTransaction> ---

    #[test]
    fn new_transaction_starts_in_normal_state() {
        let tx = make_deposit_tx(1, 1, Decimal::from(100));
        assert!(matches!(tx.state, TransactionState::Normal));
    }

    #[test]
    fn withdrawal_transaction_starts_in_normal_state() {
        let tx = make_withdrawal_tx(2, 5, Decimal::from(50));
        assert!(matches!(tx.state, TransactionState::Normal));
    }

    // --- valid_state_transition: Normal -> * ---

    #[test]
    fn normal_to_disputed_is_valid() {
        let tx = make_deposit_tx(1, 1, Decimal::ONE);
        assert!(tx.valid_state_transition(TransactionState::Disputed));
    }

    #[test]
    fn normal_to_normal_is_invalid() {
        let tx = make_deposit_tx(1, 1, Decimal::ONE);
        assert!(!tx.valid_state_transition(TransactionState::Normal));
    }

    #[test]
    fn normal_to_charged_back_is_invalid() {
        let tx = make_deposit_tx(1, 1, Decimal::ONE);
        assert!(!tx.valid_state_transition(TransactionState::ChargedBack));
    }

    // --- valid_state_transition: Disputed -> * ---

    #[test]
    fn disputed_to_normal_is_valid() {
        let mut tx = make_deposit_tx(1, 1, Decimal::ONE);
        tx.state = TransactionState::Disputed;
        assert!(tx.valid_state_transition(TransactionState::Normal));
    }

    #[test]
    fn disputed_to_charged_back_is_valid() {
        let mut tx = make_deposit_tx(1, 1, Decimal::ONE);
        tx.state = TransactionState::Disputed;
        assert!(tx.valid_state_transition(TransactionState::ChargedBack));
    }

    #[test]
    fn disputed_to_disputed_is_invalid() {
        let mut tx = make_deposit_tx(1, 1, Decimal::ONE);
        tx.state = TransactionState::Disputed;
        assert!(!tx.valid_state_transition(TransactionState::Disputed));
    }

    // --- valid_state_transition: ChargedBack -> * ---

    #[test]
    fn charged_back_to_normal_is_invalid() {
        let mut tx = make_deposit_tx(1, 1, Decimal::ONE);
        tx.state = TransactionState::ChargedBack;
        assert!(!tx.valid_state_transition(TransactionState::Normal));
    }

    #[test]
    fn charged_back_to_disputed_is_invalid() {
        let mut tx = make_deposit_tx(1, 1, Decimal::ONE);
        tx.state = TransactionState::ChargedBack;
        assert!(!tx.valid_state_transition(TransactionState::Disputed));
    }

    #[test]
    fn charged_back_to_charged_back_is_invalid() {
        let mut tx = make_deposit_tx(1, 1, Decimal::ONE);
        tx.state = TransactionState::ChargedBack;
        assert!(!tx.valid_state_transition(TransactionState::ChargedBack));
    }

    // --- state machine lifecycle ---

    #[test]
    fn full_dispute_resolve_cycle() {
        let mut tx = make_deposit_tx(1, 1, Decimal::from(100));
        assert!(matches!(tx.state, TransactionState::Normal));

        let ok = tx.valid_state_transition(TransactionState::Disputed);
        assert!(ok);
        tx.state = TransactionState::Disputed;

        let ok = tx.valid_state_transition(TransactionState::Normal);
        assert!(ok);
    }

    #[test]
    fn full_dispute_chargeback_cycle() {
        let mut tx = make_deposit_tx(1, 1, Decimal::from(100));
        assert!(matches!(tx.state, TransactionState::Normal));

        let ok = tx.valid_state_transition(TransactionState::Disputed);
        assert!(ok);
        tx.state = TransactionState::Disputed;

        let ok = tx.valid_state_transition(TransactionState::ChargedBack);
        assert!(ok);
    }

    #[test]
    fn cannot_dispute_after_chargeback() {
        let mut tx = make_deposit_tx(1, 1, Decimal::from(100));
        tx.state = TransactionState::ChargedBack;
        assert!(!tx.valid_state_transition(TransactionState::Disputed));
    }
}

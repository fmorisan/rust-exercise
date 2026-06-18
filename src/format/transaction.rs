use rust_decimal::Decimal;
use serde::{Deserialize};

#[derive(Debug)]
pub enum ParsedTransaction {
    Deposit {
        client: u16,
        id: u32,
        amount: Decimal,
    },
    Withdrawal {
        client: u16,
        id: u32,
        amount: Decimal,
    },
    Dispute {
        client: u16,
        id: u32,
    },
    Resolve {
        client: u16,
        id: u32,
    },
    Chargeback {
        client: u16,
        id: u32,
    },
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum TransactionKind {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Deserialize, Debug)]
pub struct TransactionRow {
    #[serde(rename = "type")]
    kind: TransactionKind,
    /// Client associated with this transaction
    client: u16,
    /// Transaction amount
    amount: Option<Decimal>,
    #[serde(rename = "tx")]
    /// Transaction identifier. Not sequential
    id: u32
}

pub struct ParseError;

impl TryFrom<TransactionRow> for ParsedTransaction {
    type Error = ParseError;

    fn try_from(value: TransactionRow) -> Result<Self, Self::Error> {
        match value.kind {
            TransactionKind::Deposit => {
                if let Some(amount) = value.amount {
                    return Ok(Self::Deposit { client: value.client, id: value.id, amount: amount });
                }
                Err(ParseError)
            },
            TransactionKind::Withdrawal => {
                if let Some(amount) = value.amount {
                    return Ok(Self::Withdrawal { client: value.client, id: value.id, amount: amount });
                }
                Err(ParseError)
            },
            TransactionKind::Dispute => {
                Ok(Self::Dispute { client: value.client, id: value.id })
            },
            TransactionKind::Resolve => {
                Ok(Self::Resolve { client: value.client, id: value.id })
            },
            TransactionKind::Chargeback => {
                Ok(Self::Chargeback { client: value.client, id: value.id })
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_deposit() {
        assert!(matches!(
            ParsedTransaction::try_from(
                TransactionRow {
                    kind: TransactionKind::Deposit,
                    amount: Some(Decimal::ONE),
                    client: 1,
                    id: 1
                }
            ),
            Ok(ParsedTransaction::Deposit { .. })
        ));
    }

    #[test]
    fn parse_deposit_no_amount_fails() {
        assert!(ParsedTransaction::try_from(
            TransactionRow {
                kind: TransactionKind::Deposit,
                amount: None,
                client: 1,
                id: 1
            }
        ).is_err());
    }

    #[test]
    fn parse_withdraw() {
        assert!(matches!(
            ParsedTransaction::try_from(
                TransactionRow {
                    kind: TransactionKind::Withdrawal,
                    amount: Some(Decimal::ONE),
                    client: 1,
                    id: 1
                }
            ),
            Ok(ParsedTransaction::Withdrawal { .. })
        ));
    }

    #[test]
    fn parse_withdraw_no_amount_fails() {
        assert!(ParsedTransaction::try_from(
            TransactionRow {
                kind: TransactionKind::Deposit,
                amount: None,
                client: 1,
                id: 1
            }
        ).is_err());
    }

    #[test]
    fn parse_dispute() {
        assert!(matches!(
            ParsedTransaction::try_from(
                TransactionRow {
                    kind: TransactionKind::Dispute,
                    amount: None,
                    client: 1,
                    id: 1
                }
            ),
            Ok(ParsedTransaction::Dispute { .. })
        ));
    }

    #[test]
    fn parse_resolve() {
        assert!(matches!(
            ParsedTransaction::try_from(
                TransactionRow {
                    kind: TransactionKind::Resolve,
                    amount: None,
                    client: 1,
                    id: 1
                }
            ),
            Ok(ParsedTransaction::Resolve { .. })
        ));
    }

    #[test]
    fn parse_chargeback() {
        assert!(matches!(
            ParsedTransaction::try_from(
                TransactionRow {
                    kind: TransactionKind::Chargeback,
                    amount: None,
                    client: 1,
                    id: 1
                }
            ),
            Ok(ParsedTransaction::Chargeback { .. })
        ));
    }
}

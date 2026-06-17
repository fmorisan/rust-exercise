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
    client: u16,
    amount: Option<Decimal>,
    #[serde(rename = "tx")]
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

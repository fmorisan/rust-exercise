use rust_decimal::Decimal;
use serde::Serialize;

#[derive(Serialize)]
pub struct Account {
    client: u16,
    total: Decimal,
    held: Decimal,
    available: Decimal,
    locked: bool
}

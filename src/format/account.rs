use rust_decimal::Decimal;
use serde::Serialize;

use crate::engine::account::Account as EngineAccount;

#[derive(Serialize)]
pub struct Account {
    client: u16,
    total: Decimal,
    held: Decimal,
    available: Decimal,
    locked: bool
}

impl Account {
    pub fn from_engine(client: u16, engine: &EngineAccount) -> Self {
        Account {
            client,
            total: engine.total(),
            held: engine.held(),
            available: engine.available(),
            locked: engine.locked()
        }
    }
}

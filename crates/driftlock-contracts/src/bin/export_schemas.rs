#![allow(missing_docs)]

use anyhow::Result;
use std::env;

fn main() -> Result<()> {
    let out = env::args().nth(1).unwrap_or_else(|| "contracts/schemas".to_string());
    driftlock_contracts::write_schemas(out)
}

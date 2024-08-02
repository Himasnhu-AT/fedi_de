#![allow(dead_code)]
use rusqlite::{Connection, Result};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct FederationInfo {
    pegged_in: f64,
    pegged_out: f64,
    current_balance: f64,
}

fn main() -> Result<()> {
    let conn = Connection::open("fedimint-observer.db")?;

    // Print the schema of the database
    // print_schema(&conn)?;

    // Print distinct values to understand values stored
    // print_transaction_output_kinds(&conn)?;
    // print_transaction_input_kinds(&conn)?;
    // brute_force_check_text_fields(&conn)?;

    // Calculate pegged in and pegged out amounts
    let fedration_info = get_federation_info(&conn)?;
    println!("Federation info: {:?}", fedration_info);

    Ok(())
}

fn print_schema(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("SELECT sql FROM sqlite_master WHERE type='table'")?;
    let mut rows = stmt.query([])?;

    println!("Database schema:");
    while let Some(row) = rows.next()? {
        let sql: String = row.get(0)?;
        println!("{}", sql);
    }

    Ok(())
}

fn get_federation_info(conn: &Connection) -> Result<FederationInfo> {
    // Query to get the total pegged-in amount (mint tokens created)
    let pegged_in: f64 = conn.query_row(
        "SELECT IFNULL(SUM(amount_msat), 0) FROM transaction_outputs WHERE kind = 'mint'",
        [],
        |row| row.get(0),
    )?;

    // Query to get the total pegged-out amount (mint tokens burned)
    let pegged_out: f64 = conn.query_row(
        "SELECT IFNULL(SUM(amount_msat), 0) FROM transaction_inputs WHERE kind = 'mint'",
        [],
        |row| row.get(0),
    )?;

    // Calculate the current balance in BTC (1BTC = 100_000_000_000 msat (mili-satoshi) as per https://bitcoindata.science/bitcoin-units-converter)
    let conversion_rate = 100_000_000_000.0;

    let current_balance = (pegged_in - pegged_out) / conversion_rate; // Convert from msat to BTC

    Ok(FederationInfo {
        pegged_in: pegged_in / conversion_rate,
        pegged_out: pegged_out / conversion_rate,
        current_balance,
    })
}

fn print_transaction_output_kinds(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("SELECT DISTINCT kind FROM transaction_outputs")?;
    let mut rows = stmt.query([])?;

    println!("Distinct values of 'kind' in 'transaction_outputs' table:");
    while let Some(row) = rows.next()? {
        let kind: String = row.get(0)?;
        println!("{}", kind);
    }

    Ok(())
}

fn print_transaction_input_kinds(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("SELECT DISTINCT kind FROM transaction_inputs")?;
    let mut rows = stmt.query([])?;

    println!("Distinct values of 'kind' in 'transaction_inputs' table:");
    while let Some(row) = rows.next()? {
        let kind: String = row.get(0)?;
        println!("{}", kind);
    }

    Ok(())
}

fn brute_force_check_text_fields(conn: &Connection) -> Result<()> {
    // Define the tables and their text fields
    let text_fields = vec![
        ("transaction_outputs", "kind"),
        ("transaction_inputs", "kind"),
        ("transaction_outputs", "ln_contract_interaction_kind"),
        ("ln_contracts", "type"),
    ];

    for (table, field) in text_fields {
        print_distinct_values(conn, table, field)?;
    }

    Ok(())
}

fn print_distinct_values(conn: &Connection, table: &str, field: &str) -> Result<()> {
    let query = format!("SELECT DISTINCT {} FROM {}", field, table);
    let mut stmt = conn.prepare(&query)?;
    let mut rows = stmt.query([])?;

    println!("Distinct values of '{}' in '{}' table:", field, table);
    while let Some(row) = rows.next()? {
        let value: Option<String> = row.get(0)?;
        println!("{}", value.unwrap_or_else(|| "NULL".to_string()));
    }

    Ok(())
}

This repo tracks all the approaches I took to understand the data in the `fedimint-observer.db` file.

---

## Approach one:

> :warning: For testing purposes only. Now changing it to running it with rust and build out server out of it.

Loaded the data from file, loaded in docker image of sqlite, the ran `sqlite` queries to understand the data further. How it is stored, so on...

---

## Approach two:

- [x] Rust function to extract schema from the database, though it also available in the [fedimint-observer/schema/v0.sql](https://github.com/douglaz/fedimint-observer/blob/master/schema/v0.sql) table.

<details>
<summary> Function used (rust code) </summary>

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

</details>

<details>
<summary>Output:</summary>

      Database schema:
      CREATE TABLE federations (
          federation_id BLOB PRIMARY KEY NOT NULL,
          config BLOB NOT NULL
      )
      CREATE TABLE sessions (
          federation_id BLOB NOT NULL REFERENCES federations(federation_id),
          session_index INTEGER NOT NULL,
          -- TODO: add transaction and item count
          session BLOB NOT NULL,
          PRIMARY KEY (federation_id, session_index)
      )
      CREATE TABLE transactions (
          txid BLOB NOT NULL,
          federation_id BLOB NOT NULL REFERENCES federations(federation_id),
          session_index INTEGER NOT NULL,
          item_index INTEGER NOT NULL,
          data BLOB NOT NULL,
          FOREIGN KEY (federation_id, session_index) REFERENCES sessions(federation_id, session_index),
          PRIMARY KEY (federation_id, txid)
      )
      CREATE TABLE transaction_inputs (
          federation_id BLOB NOT NULL REFERENCES federations(federation_id),
          txid BLOB NOT NULL,
          in_index INTEGER NOT NULL,
          kind TEXT NOT NULL,
          ln_contract_id BLOB,
          amount_msat INTEGER,
          PRIMARY KEY (federation_id, txid, in_index),
          FOREIGN KEY (federation_id, txid) REFERENCES transactions(federation_id, txid), -- This might be a bit too heavy of a foreign key? Maybe use rowid instead?
          FOREIGN KEY (federation_id, ln_contract_id) REFERENCES ln_contracts(federation_id, contract_id)
      )
      CREATE TABLE transaction_outputs (
          federation_id BLOB NOT NULL REFERENCES federations(federation_id),
          txid BLOB NOT NULL,
          out_index INTEGER NOT NULL,
          kind TEXT NOT NULL,
          -- We keep the ln contract relation denormalized for now. If additional modules need extra data attached to
          -- inputs/outputs we'll have to refactor that or introduce some constraints to keep the complexity manageable.
          ln_contract_interaction_kind TEXT CHECK (ln_contract_interaction_kind IN ('fund', 'cancel', 'offer', NULL)),
          ln_contract_id BLOB,
          amount_msat INTEGER,
          PRIMARY KEY (federation_id, txid, out_index),
          FOREIGN KEY (federation_id, txid) REFERENCES transactions(federation_id, txid) -- This might be a bit too heavy of a foreign key? Maybe use rowid instead?
          -- Can't apply the following FK constraint because contract doesn't exist yet when offers are created:
          -- FOREIGN KEY (federation_id, ln_contract_id) REFERENCES ln_contracts(federation_id, contract_id)
      )
      CREATE TABLE ln_contracts (
          federation_id BLOB NOT NULL REFERENCES federations(federation_id),
          contract_id BLOB NOT NULL,
          type TEXT NOT NULL CHECK (type IN ('incoming', 'outgoing')),
          payment_hash BLOB NOT NULL,
          PRIMARY KEY (federation_id, contract_id)
      )
      CREATE TABLE block_times (
          block_height INTEGER PRIMARY KEY,
          timestamp INTEGER NOT NULL
      )
      CREATE TABLE block_height_votes (
          federation_id BLOB NOT NULL REFERENCES federations(federation_id),
          session_index INTEGER NOT NULL,
          item_index INTEGER NOT NULL,
          proposer INTEGER NOT NULL,
          height_vote INTEGER NOT NULL REFERENCES block_times(block_height),
          PRIMARY KEY (federation_id, session_index, item_index),
          FOREIGN KEY (federation_id, session_index) REFERENCES sessions(federation_id, session_index)
      )

</details>

---

- [x] Rust function to brute force the unique values in the database.

<details>
<summary> Function used (rust code) </summary>
> Just a template, their were other function along with it.

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

</details>

<details>
<summary>Output:</summary>

    Distinct values of 'kind' in 'transaction_outputs' table:
    ln
    mint
    stability_pool
    wallet
    Distinct values of 'kind' in 'transaction_inputs' table:
    ln
    mint
    stability_pool
    wallet
    Distinct values of 'ln_contract_interaction_kind' in 'transaction_outputs' table:
    NULL
    offer
    fund
    cancel
    Distinct values of 'type' in 'ln_contracts' table:
    incoming
    outgoing

</details>

---

- [x] understanding `pegged_in` and `pegged_out` values.
> what pegged_in means?
> > `Bitcoin` transferred in Fedimint network, and equivlent `mint` token created in the network.

> what pegged_out means?
> > Fedration's `mint token` burned and equivalent `Bitcoin` transferred out of the network.

Going with this information, and database schema, we can assume:

- transaction_outputs table:
  - ln: Likely related to `Lightning Network` transactions, which might be part of the pegging process, especially as the fedimint utilizes it. [Reference](https://river.com/learn/terms/f/fedimint/#:~:text=Federations%20also%20make%20use%20of,bitcoin%20custody%20securely%20and%20honestly.)
  - mint: This is the most likely candidate for `pegged-in` transactions, as it suggests the creation of new tokens.
  - stability_pool: This might be related to mechanisms for stabilizing the token's value, but it's less likely to be directly tied to pegging.
  - wallet: General wallet transactions, probably not directly related to pegging.
- transaction_inputs table:
  - ln: Similar to the outputs table, likely related to Lightning Network transactions.
  - mint: This could be related to burning tokens, which might be part of the pegging-out process.
  - stability_pool: Again, related to stability mechanisms.
  - wallet: General wallet transactions.

Based on Above assumptions, We ran code:
<details>
<summary> Function used (rust code) </summary>

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
</details>

<details>
<summary>Output:</summary>

    Federation info: FederationInfo { pegged_in: 16.93090330129, pegged_out: 16.32661183985, current_balance: 0.60429146144 }

</details>

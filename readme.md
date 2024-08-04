# fedi-de

This repo tracks all the approaches I took to understand the data in the `fedimint-observer.db` file.

Expected Output:

```
Federation info: FederationInfo { pegged_in: 16.93090330129, pegged_out: 16.32661183985, current_balance: 0.60429146144 }
```

Flow i used to come to this conclusion:

- Understand schema and values using [`fedimint-observer/schema/v0`](https://github.com/Himasnhu-AT/fedimint-observer/blob/master/schema/v0.sql), api calls using `fedimint-observer`

- Ran SQL queries to extract relevant information from the database. Code of which are given in [src/main.rs](src/main.rs)

- Used the extracted information to calculate the expected output.

> As i had few doubts, went deep into codebase and found this:

```rust
process_transaction(
        dbtx: &mut Transaction<'_, Any>,
        federation_id: FederationId,
        config: &ClientConfig,
        session_index: u64,
        item_index: u64,
        transaction: fedimint_core::transaction::Transaction,
    ) -> sqlx::Result<()> {
        let txid = transaction.tx_hash();

        query("INSERT INTO transactions VALUES ($1, $2, $3, $4, $5)")
            .bind(txid.consensus_encode_to_vec())
            .bind(federation_id.consensus_encode_to_vec())
            .bind(session_index as i64)
            .bind(item_index as i64)
            .bind(transaction.consensus_encode_to_vec())
            .execute(dbtx.as_mut())
            .await?;

        for (in_idx, input) in transaction.inputs.into_iter().enumerate() {
            let kind = instance_to_kind(config, input.module_instance_id());
            let (maybe_amount_msat, maybe_ln_contract_id) = match kind.as_str() {
                "ln" => {
                    let input = input
                        .as_any()
                        .downcast_ref::<LightningInput>()
                        .expect("Not LN input")
                        .maybe_v0_ref()
                        .expect("Not v0");

                    (Some(input.amount.msats), Some(input.contract_id))
                }
                "mint" => {
                    let amount_msat = input
                        .as_any()
                        .downcast_ref::<MintInput>()
                        .expect("Not Mint input")
                        .maybe_v0_ref()
                        .expect("Not v0")
                        .amount
                        .msats;

                    (Some(amount_msat), None)
                }
                "wallet" => {
                    let amount_msat = input
                        .as_any()
                        .downcast_ref::<WalletInput>()
                        .expect("Not Wallet input")
                        .maybe_v0_ref()
                        .expect("Not v0")
                        .0
                        .tx_output()
                        .value
                        * 1000;
                    (Some(amount_msat), None)
                }
                _ => (None, None),
            };

            query("INSERT INTO transaction_inputs VALUES ($1, $2, $3, $4, $5, $6)")
                .bind(federation_id.consensus_encode_to_vec())
                .bind(txid.consensus_encode_to_vec())
                .bind(in_idx as i64)
                .bind(kind.as_str())
                .bind(maybe_ln_contract_id.map(|cid| cid.consensus_encode_to_vec()))
                .bind(maybe_amount_msat.map(|amt| amt as i64))
                .execute(dbtx.as_mut())
                .await?;
        }

        for (out_idx, output) in transaction.outputs.into_iter().enumerate() {
            let kind = instance_to_kind(config, output.module_instance_id());
            let (maybe_amount_msat, maybe_ln_contract) = match kind.as_str() {
                "ln" => {
                    let ln_output = output
                        .as_any()
                        .downcast_ref::<LightningOutput>()
                        .expect("Not LN input")
                        .maybe_v0_ref()
                        .expect("Not v0");
                    let (maybe_amount_msat, ln_contract_interaction_kind, contract_id) =
                        match ln_output {
                            LightningOutputV0::Contract(contract) => {
                                let contract_id = contract.contract.contract_id();
                                let (contract_type, payment_hash) = match &contract.contract {
                                    Contract::Incoming(c) => ("incoming", c.hash),
                                    Contract::Outgoing(c) => ("outgoing", c.hash),
                                };

                                query("INSERT INTO ln_contracts VALUES ($1, $2, $3, $4)")
                                    .bind(federation_id.consensus_encode_to_vec())
                                    .bind(contract_id.consensus_encode_to_vec())
                                    .bind(contract_type)
                                    .bind(payment_hash.consensus_encode_to_vec())
                                    .execute(dbtx.as_mut())
                                    .await?;

                                (Some(contract.amount.msats), "fund", contract_id)
                            }
                            LightningOutputV0::Offer(offer) => {
                                // For incoming contracts payment has == cotnract id
                                (Some(0), "offer", offer.hash.into())
                            }
                            LightningOutputV0::CancelOutgoing { contract, .. } => {
                                (Some(0), "cancel", *contract)
                            }
                        };

                    (
                        maybe_amount_msat,
                        Some((ln_contract_interaction_kind, contract_id)),
                    )
                }
                "mint" => {
                    let amount_msat = output
                        .as_any()
                        .downcast_ref::<MintOutput>()
                        .expect("Not Mint input")
                        .maybe_v0_ref()
                        .expect("Not v0")
                        .amount
                        .msats;
                    (Some(amount_msat), None)
                }
                "wallet" => {
                    let amount_msat = output
                        .as_any()
                        .downcast_ref::<WalletOutput>()
                        .expect("Not Wallet input")
                        .maybe_v0_ref()
                        .expect("Not v0")
                        .amount()
                        .to_sat()
                        * 1000;
                    (Some(amount_msat), None)
                }
                _ => (None, None),
            };

            query("INSERT INTO transaction_outputs VALUES ($1, $2, $3, $4, $5, $6, $7)")
                .bind(federation_id.consensus_encode_to_vec())
                .bind(txid.consensus_encode_to_vec())
                .bind(out_idx as i64)
                .bind(kind.as_str())
                .bind(maybe_ln_contract.map(|cd| cd.0))
                .bind(maybe_ln_contract.map(|cd| cd.1.consensus_encode_to_vec()))
                .bind(maybe_amount_msat.map(|amt| amt as i64))
                .execute(dbtx.as_mut())
                .await?;
        }

        Ok(())
    }
```

This helped me ensure correct results.

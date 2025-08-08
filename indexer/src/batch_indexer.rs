use std::str::FromStr;

use alloy::{
    primitives::Address,
    providers::{DynProvider, Provider, ProviderBuilder},
    rpc::types::Filter,
    sol_types::SolEvent,
};
use anyhow::Error;

use crate::{config::Config, db::DataBase};

use super::taiko_inbox_binding::ITaikoInbox;

use tokio::time::{Duration, sleep};

pub struct BatchIndexer {
    indexed_l1_block: u64,
    db: DataBase,
    l1_provider: DynProvider,
    l2_provider: DynProvider,
    taiko_inbox: Address,
    proving_window: u64,
    indexing_step: u64,
    sleep_duration_sec: u64,
}

impl BatchIndexer {
    pub async fn new(config: Config) -> Result<Self, Error> {
        let db = DataBase::new(&config.db_filename).await?;
        let indexed_l1_block = db.get_indexed_l1_block().await.max(config.l1_start_block);
        let l1_provider = ProviderBuilder::new()
            .connect_http(config.l1_rpc_url.parse()?)
            .erased();

        let taiko_inbox = Address::from_str(config.taiko_inbox_address.as_str())?;

        let ti_contract = ITaikoInbox::new(taiko_inbox, &l1_provider);
        let pacaya_config = ti_contract.pacayaConfig().call().await?;
        tracing::info!("Proving window: {}", pacaya_config.provingWindow);

        let l2_provider = ProviderBuilder::new()
            .connect_http(config.l2_rpc_url.parse()?)
            .erased();

        Ok(Self {
            indexed_l1_block,
            db,
            l1_provider,
            l2_provider,
            taiko_inbox,
            proving_window: u64::from(pacaya_config.provingWindow),
            indexing_step: config.indexing_step,
            sleep_duration_sec: config.sleep_duration_sec,
        })
    }

    pub async fn run_indexing_loop(&mut self) {
        loop {
            let current_block = match self.l1_provider.get_block_number().await {
                Ok(block) => block,
                Err(e) => panic!("Failed to get current block number: {e}"),
            };
            let from_block = self.indexed_l1_block + 1;
            let to_block = from_block + self.indexing_step;
            tracing::info!("Indexing from block {from_block} to block {to_block}");
            if current_block > to_block {
                let (proposed_batch_id, proposed_block_id) = self
                    .index_batch_proposed(from_block, to_block)
                    .await
                    .unwrap_or_else(|e| {
                        panic!(
                            "Failed to index BatchProposed event (from: {from_block}, to: {to_block}): {e}"
                        )
                    });
                let (proved_batch_id, proved_block_id) = self
                    .index_batch_proved(from_block, to_block)
                    .await
                    .unwrap_or_else(|e| {
                        panic!(
                            "Failed to index BatchesProved event (from: {from_block}, to: {to_block}): {e}"
                        )
                    });

                self.indexed_l1_block = to_block;

                if let Err(e) = self
                    .db
                    .update_status(
                        self.indexed_l1_block,
                        proposed_batch_id,
                        proposed_block_id,
                        proved_batch_id,
                        proved_block_id,
                    )
                    .await
                {
                    tracing::error!("Failed to update status: {}", e);
                }
            }

            let current_block = match self.l1_provider.get_block_number().await {
                Ok(block) => block,
                Err(e) => panic!("Failed to get current block number: {e}"),
            };
            if self.indexed_l1_block + self.indexing_step > current_block {
                sleep(Duration::from_secs(
                    self.indexing_step * self.sleep_duration_sec,
                ))
                .await;
            } else {
                sleep(Duration::from_secs(self.sleep_duration_sec)).await;
            }
        }
    }

    pub async fn index_batch_proposed(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<(u64, u64), Error> {
        let batch_proposed = ITaikoInbox::BatchProposed::SIGNATURE_HASH;
        let filter = Filter::new()
            .address(self.taiko_inbox)
            .event_signature(batch_proposed)
            .from_block(from_block)
            .to_block(to_block);
        let logs = self.l1_provider.get_logs(&filter).await?;
        tracing::debug!("Found {} BatchProposed Events", logs.len());

        let mut propsed_batch_id = 0;
        let mut proposed_block_id = 0;

        for log in logs {
            let receipt = match self
                .l1_provider
                .get_transaction_receipt(log.transaction_hash.expect("Transaction receipt not found"))
                .await?
            {
                Some(receipt) => receipt,
                None => panic!(
                    "Transaction receipt not found for {:?}",
                    log.transaction_hash
                ),
            };
            let propose_fee = Self::get_tx_eth_price(&receipt);
            let batch = log.log_decode::<ITaikoInbox::BatchProposed>()?;

            propsed_batch_id = propsed_batch_id.max(batch.inner.meta.batchId);
            proposed_block_id = proposed_block_id.max(batch.inner.info.lastBlockId);
            self.db
                .insert_batch(
                    batch,
                    log.transaction_hash.expect("proposeBatch transaction hash not found").to_string(),
                    receipt.from,
                    propose_fee,
                )
                .await?;
        }

        Ok((propsed_batch_id, proposed_block_id))
    }

    pub async fn index_batch_proved(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<(u64, u64), Error> {
        let batches_proved = ITaikoInbox::BatchesProved::SIGNATURE_HASH;
        let filter = Filter::new()
            .address(self.taiko_inbox)
            .event_signature(batches_proved)
            .from_block(from_block)
            .to_block(to_block);
        let logs = self.l1_provider.get_logs(&filter).await?;
        tracing::debug!("Found {} BatchesProved Events", logs.len());

        let mut proved_batch_id = 0;
        let mut proved_block_id = 0u64;

        for log in logs {
            let batches = log.log_decode::<ITaikoInbox::BatchesProved>()?;
            let receipt = self
                .l1_provider
                .get_transaction_receipt(log.transaction_hash.expect("proveBatch transaction receipt not found"))
                .await?
                .expect("proveBatch transaction receipt is None");
            tracing::debug!("Proved {} batches", batches.inner.batchIds.len());

            let tx_hash = log.transaction_hash.expect("proveBatch transaction hash not found").to_string();
            // we divide the total fee by the number of batches to get the prove fee
            let prove_fee = Self::get_tx_eth_price(&receipt) / batches.inner.batchIds.len() as u128;

            for batch_id in batches.inner.batchIds.clone() {
                if let Some(mut batch) = self.db.get_batch_by_id(batch_id.try_into()?).await {
                    proved_batch_id = proved_batch_id.max(batch_id);
                    proved_block_id = proved_block_id.max(batch.last_block_id.try_into()?);

                    batch.prove_tx = Some(tx_hash.clone());
                    batch.prove_fee = Some(prove_fee.to_string());
                    let prover = self
                        .get_prover(
                            receipt.from.to_string().as_str(),
                            batch.sender.as_str(),
                            receipt.block_number.expect("receipt block number is None"),
                            batch.proposed_at.try_into()?,
                        )
                        .await?;
                    batch.is_proved_by_proposer = Some(prover == batch.proposer);
                    batch.prover = Some(prover);
                    let l2_fee_earned = self
                        .calculate_l2_fee_earned(
                            batch.coinbase.as_str(),
                            batch.last_block_id.try_into()?,
                            batch.block_count.try_into()?,
                        )
                        .await?;
                    batch.l2_fee_earned = Some(l2_fee_earned.to_string());
                    batch.is_profitable = Some(
                        l2_fee_earned
                            > prove_fee
                                + batch
                                    .propose_fee
                                    .parse::<u128>()
                                    .expect("Failed to parse propose fee {}"),
                    );

                    self.db.update_batch(batch).await?;
                } else {
                    tracing::error!("Batch with id {} not found", batch_id);
                }
            }
        }
        Ok((proved_batch_id, proved_block_id))
    }

    async fn calculate_l2_fee_earned(
        &self,
        coinbase_address: &str,
        last_block_number: u64,
        block_count: u64,
    ) -> Result<u128, Error> {
        let coinbase: Address = Address::from_str(coinbase_address)?;

        let start_block = last_block_number - block_count;

        let balance_before = self
            .l2_provider
            .get_balance(coinbase)
            .block_id(start_block.into())
            .await?;

        let balance_after = self
            .l2_provider
            .get_balance(coinbase)
            .block_id(last_block_number.into())
            .await?;

        Ok((balance_after - balance_before).try_into()?)
    }

    async fn get_prover(
        &self,
        prove_sender: &str,
        propose_sender: &str,
        prove_block: u64,
        proposed_at: u64,
    ) -> Result<String, Error> {
        if let Some(block) = self
            .l1_provider
            .get_block_by_number(prove_block.into())
            .await?
        {
            if block.header.inner.timestamp > proposed_at + self.proving_window {
                Ok(prove_sender.to_string())
            } else {
                Ok(propose_sender.to_string())
            }
        } else {
            panic!("Prove block {prove_block} not found");
        }
    }

    fn get_tx_eth_price(receipt: &alloy::rpc::types::TransactionReceipt) -> u128 {
        let gas_used = u128::from(receipt.gas_used);
        let gas_price = receipt.effective_gas_price;
        let blob_gas_used = u128::from(receipt.blob_gas_used.unwrap_or(0));
        let blob_gas_price = receipt.blob_gas_price.unwrap_or(0);

        let execution_fee = gas_used * gas_price;
        let blob_fee = blob_gas_used * blob_gas_price;

        execution_fee + blob_fee
    }
}

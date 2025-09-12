use alloy::{primitives::Address, rpc::types::Log};
use anyhow::Error;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};

use crate::taiko_inbox_binding::ITaikoInbox;

#[allow(dead_code)]
#[derive(sqlx::FromRow)]
pub struct Batch {
    pub batch_id: i64,
    pub sender: String,
    pub proposer: String,
    pub coinbase: String,
    pub propose_tx: String,
    pub proposed_at: i64,
    pub last_block_id: i64,
    pub block_count: i64,
    pub propose_fee: String,
    pub l2_fee_earned: Option<String>,
    pub prover: Option<String>,
    pub prove_tx: Option<String>,
    pub prove_fee: Option<String>,
    pub is_sent_by_proposer: bool,
    pub is_profitable: Option<bool>,
    pub is_proved_by_proposer: Option<bool>,
}

pub struct DataBase {
    pool: SqlitePool,
}

impl DataBase {
    pub async fn new(db_filename: &str) -> Result<Self, Error> {
        let options = SqliteConnectOptions::new()
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .filename(db_filename)
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;

        // Create batch table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS batch (
                batch_id              INTEGER PRIMARY KEY,
                sender                TEXT NOT NULL,
                proposer              TEXT NOT NULL,
                coinbase              TEXT NOT NULL,
                propose_tx            TEXT NOT NULL,
                proposed_at           INTEGER NOT NULL,
                last_block_id         INTEGER NOT NULL,
                block_count           INTEGER NOT NULL,
                propose_fee          TEXT NOT NULL,
                l2_fee_earned         TEXT,
                prover                TEXT,
                prove_tx              TEXT,
                prove_fee            TEXT,
                is_sent_by_proposer   BOOLEAN NOT NULL,
                is_profitable         BOOLEAN,
                is_proved_by_proposer BOOLEAN
            );
            CREATE INDEX IF NOT EXISTS idx_batch_proposed_at ON batch(proposed_at);
            CREATE INDEX IF NOT EXISTS idx_batch_proposer ON batch(proposer);
            CREATE INDEX IF NOT EXISTS idx_batch_profitable ON batch(is_profitable);
            CREATE INDEX IF NOT EXISTS idx_batch_sender ON batch(is_sent_by_proposer);
            CREATE INDEX IF NOT EXISTS idx_batch_proving_window ON batch(is_proved_by_proposer);
            "#,
        )
        .execute(&pool)
        .await?;

        // Create status table (only one row allowed)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS status (
                id                  INTEGER PRIMARY KEY CHECK (id = 0),
                indexed_l1_block    INTEGER NOT NULL,
                proposed_batch_id   INTEGER NOT NULL,
                proposed_block_id   INTEGER NOT NULL,
                proved_batch_id     INTEGER NOT NULL,
                proved_block_id     INTEGER NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // Insert status if not exist
        let status = sqlx::query(
            r#"
            SELECT id FROM status WHERE id = 0
            "#,
        )
        .fetch_optional(&pool)
        .await?;
        if status.is_none() {
            sqlx::query(
                r#"
                INSERT INTO status (id, indexed_l1_block, proposed_batch_id, proposed_block_id, proved_batch_id, proved_block_id)
                VALUES (0, 0, 0, 0, 0, 0)
                "#,
            )
            .execute(&pool)
            .await?;
        }

        Ok(Self { pool })
    }

    pub async fn get_indexed_l1_block(&self) -> u64 {
        sqlx::query_as(
            r#"
            SELECT indexed_l1_block FROM status WHERE id = 0
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or((0,))
        .0
        .try_into()
        .expect("Cannot convert indexed_l1_block to u64")
    }

    pub async fn update_status(
        &self,
        indexed_l1_block: u64,
        proposed_batch_id: u64,
        proposed_block_id: u64,
        proved_batch_id: u64,
        proved_block_id: u64,
    ) -> Result<(), Error> {
        let mut query = String::from("UPDATE status SET ");
        let mut updates = Vec::new();
        let mut values: Vec<i64> = Vec::new();

        if indexed_l1_block != 0 {
            updates.push("indexed_l1_block = ?");
            values.push(indexed_l1_block.try_into()?);
        }
        if proposed_batch_id != 0 {
            updates.push("proposed_batch_id = ?");
            values.push(proposed_batch_id.try_into()?);
        }
        if proposed_block_id != 0 {
            updates.push("proposed_block_id = ?");
            values.push(proposed_block_id.try_into()?);
        }
        if proved_batch_id != 0 {
            updates.push("proved_batch_id = ?");
            values.push(proved_batch_id.try_into()?);
        }
        if proved_block_id != 0 {
            updates.push("proved_block_id = ?");
            values.push(proved_block_id.try_into()?);
        }

        if updates.is_empty() {
            return Ok(()); // nothing to update
        }

        query.push_str(&updates.join(", "));
        query.push_str(" WHERE id = 0");

        let mut sql = sqlx::query(&query);
        for val in values {
            sql = sql.bind(val);
        }

        sql.execute(&self.pool).await?;
        Ok(())
    }

    pub async fn insert_batch(
        &self,
        batch: Log<ITaikoInbox::BatchProposed>,
        tx_hash: String,
        sender: Address,
        propose_fee: u128,
    ) -> Result<(), Error> {
        let batch_id: i64 = batch.inner.meta.batchId.try_into()?;
        let is_sent_by_proposer = sender == batch.inner.info.coinbase;
        let sender = sender.to_string();
        let proposer = batch.inner.meta.proposer.to_string();
        let propose_tx = tx_hash;
        let proposed_at: i64 = batch.inner.meta.proposedAt.try_into()?;
        let last_block_id: i64 = batch.inner.info.lastBlockId.try_into()?;
        let block_count: i64 = batch.inner.info.blocks.len().try_into()?;
        let propose_fee = propose_fee.to_string();
        let coinbase = batch.inner.info.coinbase.to_string();

        let result = sqlx::query(
            r#"
            INSERT INTO batch (
                batch_id, sender, proposer, coinbase, propose_tx, proposed_at,
                last_block_id, block_count, propose_fee, is_sent_by_proposer
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(batch_id)
        .bind(sender)
        .bind(proposer)
        .bind(coinbase)
        .bind(propose_tx)
        .bind(proposed_at)
        .bind(last_block_id)
        .bind(block_count)
        .bind(propose_fee)
        .bind(is_sent_by_proposer)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => tracing::debug!("Batch inserted: batch_id {}", batch_id),
            Err(sqlx::error::Error::Database(db_err)) if db_err.is_unique_violation() => {
                tracing::error!("Duplicate batch_id {}, insert skipped", batch_id);
            }
            Err(e) => return Err(e.into()),
        }

        Ok(())
    }

    pub async fn get_batch_by_id(&self, batch_id: i64) -> Option<Batch> {
        match sqlx::query_as(
            r#"
            SELECT * FROM batch WHERE batch_id = ?
            "#,
        )
        .bind(batch_id)
        .fetch_optional(&self.pool)
        .await
        {
            Ok(res) => res,
            Err(e) => {
                tracing::error!("Error getting batch by id {}: {}", batch_id, e);
                None
            }
        }
    }

    pub async fn update_batch(&self, batch: Batch) -> Result<(), Error> {
        sqlx::query(
            r#"
            UPDATE batch SET
                l2_fee_earned = ?,
                prover = ?,
                prove_tx = ?,
                prove_fee = ?,
                is_profitable = ?,
                is_proved_by_proposer = ?
            WHERE batch_id = ?
            "#,
        )
        .bind(batch.l2_fee_earned)
        .bind(batch.prover)
        .bind(batch.prove_tx)
        .bind(batch.prove_fee)
        .bind(batch.is_profitable)
        .bind(batch.is_proved_by_proposer)
        .bind(batch.batch_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

use crate::filter_batches::filter_batches;
use crate::get_accounting_list::get_accounting_list;
use crate::models::{AccountingListGql, AccountingOperation, AccountingResult, Batch, Status};
use async_graphql::{Context, Object, Schema};
use sqlx::SqlitePool;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Returns the status of the server
    async fn status(&self, ctx: &Context<'_>) -> async_graphql::Result<Status> {
        let pool = ctx.data::<SqlitePool>()?;
        let status = sqlx::query_as::<_, Status>("SELECT * FROM status WHERE id = 0")
            .fetch_one(pool)
            .await?;
        Ok(status)
    }

    /// Returns the latest batch (by ID) proposed at or before the given timestamp.
    /// `timestamp`: The cutoff timestamp (inclusive).
    async fn latest_batch_before_timestamp(
        &self,
        ctx: &Context<'_>,
        timestamp: i64,
    ) -> async_graphql::Result<Option<i64>> {
        let pool = ctx.data::<SqlitePool>()?;
        let batch_id =
            sqlx::query_scalar::<_, i64>("SELECT MAX(batch_id) FROM batch WHERE proposed_at <= ?")
                .bind(timestamp)
                .fetch_optional(pool)
                .await?;

        Ok(batch_id)
    }

    /// Computes the accounting of batch fees for a given address within a range of batch IDs.
    ///
    /// This function returns an `AccountingResult` containing:
    /// - `debit`: fees that other teams owe to the given address.
    /// - `credit`: fees that the given address owes to other teams.
    ///
    /// # Parameters
    /// - `ctx`: GraphQL context, used to access the database pool.
    /// - `address`: The address for which to compute the accounting.
    /// - `from`: Starting batch ID (inclusive) for the range.
    /// - `to`: Ending batch ID (inclusive) for the range. Must be greater than `from`.
    /// - `check_integrity`: If `true`, validates that all batch IDs in the range exist and returns an error if not.
    async fn accounting(
        &self,
        ctx: &Context<'_>,
        address: String,
        from: i64,
        to: i64,
        check_integrity: Option<bool>,
    ) -> async_graphql::Result<AccountingResult> {
        let pool = ctx.data::<SqlitePool>()?;

        if from >= to {
            return Err(async_graphql::Error::new("from must be less than to"));
        }

        if let Some(check_integrity) = check_integrity
            && check_integrity
        {
            // Count batches in the given range
            let batch_count: Option<i64> = sqlx::query_scalar(
                "SELECT COUNT(batch_id) FROM batch WHERE batch_id >= ? AND batch_id <= ?",
            )
            .bind(from)
            .bind(to)
            .fetch_optional(pool)
            .await?;

            let batch_count = batch_count.unwrap_or(0);

            if batch_count == 0 {
                return Err(async_graphql::Error::new(
                    "Integrity error: No batches found in range",
                ));
            }

            // If count does not match the expected number of batches in the range
            if batch_count != to - from + 1 {
                return Err(async_graphql::Error::new(
                    "Integrity error: Not all batches found in range",
                ));
            }
        }

        let debit =
            get_accounting_list(ctx, AccountingOperation::Debit, address.clone(), from, to).await?;
        let credit =
            get_accounting_list(ctx, AccountingOperation::Credit, address, from, to).await?;
        let res = AccountingResult {
            debit: AccountingListGql::from(debit),
            credit: AccountingListGql::from(credit),
        };
        Ok(res)
    }

    /// Returns the batch with the given id\
    /// `id`: Batch id
    async fn batch_by_id(
        &self,
        ctx: &Context<'_>,
        id: i64,
    ) -> async_graphql::Result<Option<Batch>> {
        let pool = ctx.data::<SqlitePool>()?;
        let batch = sqlx::query_as::<_, Batch>("SELECT * FROM batch WHERE batch_id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        Ok(batch)
    }

    /// Returns batches that were landed on L1 by a different party than the proposer\
    /// `proposer`: Filter by batch proposer address\
    /// `start`: Filter by proposed_at time greater than or equal to this value\
    /// `end`: Filter by proposed_at time less than or equal to this value\
    async fn sent_by_others(
        &self,
        ctx: &Context<'_>,
        proposer: Option<String>,
        sender: Option<String>,
        start: Option<i64>,
        end: Option<i64>,
    ) -> async_graphql::Result<Vec<Batch>> {
        filter_batches(ctx, "is_sent_by_proposer = 0", proposer, sender, start, end).await
    }

    /// Returns batches that were proven by a different party than the proposer\
    /// `proposer`: Filter by batch proposer address\
    /// `start`: Filter by proposed_at time greater than or equal to this value\
    /// `end`: Filter by proposed_at time less than or equal to this value\
    async fn proved_by_others(
        &self,
        ctx: &Context<'_>,
        proposer: Option<String>,
        start: Option<i64>,
        end: Option<i64>,
    ) -> async_graphql::Result<Vec<Batch>> {
        filter_batches(ctx, "is_proved_by_proposer = 0", proposer, None, start, end).await
    }

    /// Returns batches that were not profitable\
    /// `proposer`: Filter by batch proposer address\
    /// `start`: Filter by proposed_at time greater than or equal to this value\
    /// `end`: Filter by proposed_at time less than or equal to this value\
    async fn unprofitable(
        &self,
        ctx: &Context<'_>,
        proposer: Option<String>,
        start: Option<i64>,
        end: Option<i64>,
    ) -> async_graphql::Result<Vec<Batch>> {
        filter_batches(ctx, "is_profitable = 0", proposer, None, start, end).await
    }
}

pub type AppSchema =
    Schema<QueryRoot, async_graphql::EmptyMutation, async_graphql::EmptySubscription>;

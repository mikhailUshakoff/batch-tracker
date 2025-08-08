use crate::models::{Batch, Status};
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

// Extracted reusable filtering logic
async fn filter_batches(
    ctx: &Context<'_>,
    base_condition: &str,
    proposer: Option<String>,
    sender: Option<String>,
    start: Option<i64>,
    end: Option<i64>,
) -> async_graphql::Result<Vec<Batch>> {
    let pool = ctx.data::<SqlitePool>()?;
    let mut query = format!("SELECT * FROM batch WHERE {base_condition}");

    if proposer.is_some() {
        query.push_str(" AND proposer = ?");
    }
    if sender.is_some() {
        query.push_str(" AND sender = ?");
    }
    if start.is_some() {
        query.push_str(" AND proposed_at >= ?");
    }
    if end.is_some() {
        query.push_str(" AND proposed_at <= ?");
    }

    let mut q = sqlx::query_as::<_, Batch>(&query);
    if let Some(p) = proposer {
        q = q.bind(p);
    }
    if let Some(s) = sender {
        q = q.bind(s);
    }
    if let Some(s) = start {
        q = q.bind(s);
    }
    if let Some(e) = end {
        q = q.bind(e);
    }

    Ok(q.fetch_all(pool).await?)
}

pub type AppSchema =
    Schema<QueryRoot, async_graphql::EmptyMutation, async_graphql::EmptySubscription>;

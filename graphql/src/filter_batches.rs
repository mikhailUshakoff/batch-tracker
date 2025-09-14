use crate::models::Batch;
use async_graphql::Context;
use sqlx::SqlitePool;

pub async fn filter_batches(
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

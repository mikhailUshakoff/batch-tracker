use crate::models::{AccountingList, AccountingOperation, Batch};
use async_graphql::Context;
use sqlx::SqlitePool;

pub async fn get_accounting_list(
    ctx: &Context<'_>,
    operation: AccountingOperation,
    address: String,
    from: i64,
    to: i64,
) -> async_graphql::Result<AccountingList> {
    let pool = ctx.data::<SqlitePool>()?;

    let mut query = "SELECT * FROM batch WHERE".to_string();
    match operation {
        AccountingOperation::Debit => {
            query.push_str(" proposer = ? AND coinbase <> ?");
        }
        AccountingOperation::Credit => {
            query.push_str(" proposer <> ? AND coinbase = ?");
        }
    }

    query.push_str(" AND batch_id >= ? AND batch_id <= ?");

    let batches: Vec<Batch> = sqlx::query_as::<_, Batch>(&query)
        .bind(&address)
        .bind(&address)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await?;

    let mut list = AccountingList::new();

    batches
        .into_iter()
        .try_for_each(|batch| list.add_batch(&operation, batch))?;

    Ok(list)
}

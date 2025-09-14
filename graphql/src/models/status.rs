use async_graphql::SimpleObject;

#[derive(Debug, sqlx::FromRow, SimpleObject)]
pub struct Status {
    pub id: i64,
    /// Last indexed L1 block
    pub indexed_l1_block: i64,
    /// Last indexed proposed batch
    pub proposed_batch_id: i64,
    /// Last indexed proposed block
    pub proposed_block_id: i64,
    /// Last indexed proven batch
    pub proved_batch_id: i64,
    /// Last indexed proven block
    pub proved_block_id: i64,
}

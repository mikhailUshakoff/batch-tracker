use async_graphql::SimpleObject;

#[derive(Debug, sqlx::FromRow, SimpleObject)]
pub struct Batch {
    pub batch_id: i64,
    /// Batch sender to L1
    pub sender: String,
    /// Blocks preconfer
    pub proposer: String,
    /// Coinbase address on L2
    pub coinbase: String,
    /// proposeBatch transaction hash on L1
    pub propose_tx: String,
    /// Timestamp of block with proposeBatch transaction
    pub proposed_at: i64,
    /// Last block in the batch
    pub last_block_id: i64,
    /// Number of blocks in the batch
    pub block_count: i64,
    /// Fee to call proposeBatch on L1
    pub propose_fee: String,
    /// L2 fee earned for preconfirmation
    pub l2_fee_earned: Option<String>,
    /// Address wich receives TAIKO tokens after proving
    pub prover: Option<String>,
    /// proveBatch transaction hash on L1
    pub prove_tx: Option<String>,
    /// Fee to call proveBatch on L1
    pub prove_fee: Option<String>,
    /// Flag indicating if proposeBatch transaction was sent by the proposer
    pub is_sent_by_proposer: bool,
    /// Flag indecating if l2_fee_earned >= propose_fee + prove_fee
    pub is_profitable: Option<bool>,
    /// Flag indecating if TAIKO tokens were sent to proposer
    pub is_proved_by_proposer: Option<bool>,
}

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

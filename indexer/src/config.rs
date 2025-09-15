pub struct Config {
    pub db_filename: String,
    pub l1_rpc_url: String,
    pub l2_rpc_url: String,
    pub taiko_inbox_address: String,
    pub l1_start_block: u64,
    pub indexing_step: u64,
    pub sleep_duration_sec: u64,
    pub max_l1_fork_depth: u64,
}

impl Config {
    pub fn new() -> Self {
        // Load environment variables from .env file
        dotenvy::dotenv().ok();

        let db_filename = std::env::var("DB_FILENAME").unwrap_or_else(|_| {
            panic!("DB_FILENAME env var not found");
        });

        let l1_rpc_url = std::env::var("L1_RPC_URL").unwrap_or_else(|_| {
            panic!("L1_RPC_URL env var not found");
        });

        let l2_rpc_url = std::env::var("L2_RPC_URL").unwrap_or_else(|_| {
            panic!("L2_RPC_URL env var not found");
        });

        let taiko_inbox_address = std::env::var("TAIKO_INBOX_ADDRESS").unwrap_or_else(|_| {
            panic!("TAIKO_INBOX_ADDRESS env var not found");
        });

        let l1_start_block = std::env::var("L1_START_BLOCK")
            .unwrap_or("0".to_string())
            .parse::<u64>()
            .inspect(|&val| {
                if val == 0 {
                    panic!("L1_START_BLOCK must be a positive number");
                }
            })
            .expect("L1_START_BLOCK must be a number");

        let indexing_step = std::env::var("INDEXING_STEP")
            .unwrap_or("10".to_string())
            .parse::<u64>()
            .inspect(|&val| {
                if val == 0 {
                    panic!("INDEXING_STEP must be a positive number");
                }
            })
            .expect("INDEXING_STEP must be a number");
        let sleep_duration_sec = std::env::var("SLEEP_DURATION_SEC")
            .unwrap_or("12".to_string())
            .parse::<u64>()
            .inspect(|&val| {
                if val == 0 {
                    panic!("SLEEP_DURATION_SEC must be a positive number");
                }
            })
            .expect("SLEEP_DURATION_SEC must be a number");

        let max_l1_fork_depth = std::env::var("MAX_L1_FORK_DEPTH")
            .unwrap_or("10".to_string())
            .parse::<u64>()
            .inspect(|&val| {
                if val == 0 {
                    panic!("MAX_L1_FORK_DEPTH must be a positive number");
                }
            })
            .expect("MAX_L1_FORK_DEPTH must be a number");

        tracing::info!(
            "Config:\nDB_FILENAME: {}\nL1_RPC_URL: {}\nL2_RPC_URL: {}\nTAIKO_INBOX_ADDRESS: {}\nL1_START_BLOCK: {}\nINDEXING_STEP: {}\nSLEEP_DURATION_SEC: {}\nMAX_L1_FORK_DEPTH: {}",
            db_filename,
            l1_rpc_url,
            l2_rpc_url,
            taiko_inbox_address,
            l1_start_block,
            indexing_step,
            sleep_duration_sec,
            max_l1_fork_depth
        );

        Config {
            db_filename,
            l1_rpc_url,
            l2_rpc_url,
            taiko_inbox_address,
            l1_start_block,
            indexing_step,
            sleep_duration_sec,
            max_l1_fork_depth,
        }
    }
}

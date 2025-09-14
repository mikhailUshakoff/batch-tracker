use alloy::primitives::U256;
use async_graphql::SimpleObject;
use std::collections::HashMap;

use crate::models::Batch;

#[derive(Debug, SimpleObject)]
pub struct AddressInfoGql {
    address: String,
    total_fee: String,
    batches: Vec<Batch>,
}

#[derive(Debug, SimpleObject)]
pub struct AccountingResult {
    pub debit: AccountingListGql,
    pub credit: AccountingListGql,
}

#[derive(Debug, SimpleObject)]
pub struct AccountingListGql {
    total_fee: String,
    addresses: Vec<AddressInfoGql>,
}

pub struct AddressInfo {
    total_fee: U256,
    batches: Vec<Batch>,
}

impl AddressInfoGql {
    pub fn new(address: String, total_fee: U256, batches: Vec<Batch>) -> Self {
        Self {
            address,
            total_fee: wei_to_eth_string(&total_fee),
            batches,
        }
    }
}

impl AddressInfo {
    pub fn new() -> Self {
        AddressInfo {
            total_fee: U256::ZERO,
            batches: Vec::new(),
        }
    }

    pub fn add_batch(&mut self, fee: U256, batch: Batch) {
        self.total_fee += fee;
        self.batches.push(batch);
    }
}

pub struct AccountingList {
    total_fee: U256,
    addresses: HashMap<String, AddressInfo>,
}

impl From<AccountingList> for AccountingListGql {
    fn from(list: AccountingList) -> Self {
        let addresses = list
            .addresses
            .into_iter()
            .map(|(k, v)| AddressInfoGql::new(k, v.total_fee, v.batches))
            .collect::<Vec<_>>();

        Self {
            total_fee: wei_to_eth_string(&list.total_fee),
            addresses,
        }
    }
}

impl AccountingList {
    pub fn new() -> Self {
        AccountingList {
            total_fee: U256::ZERO,
            addresses: HashMap::new(),
        }
    }

    pub fn add_batch(
        &mut self,
        operation: &AccountingOperation,
        batch: Batch,
    ) -> async_graphql::Result<()> {
        let fee = U256::from_str_radix(&batch.propose_fee, 10)
            .map_err(|e| async_graphql::Error::new(format!("Cannot parse propose fee: {e}")))?;
        let key = match operation {
            AccountingOperation::Debit => batch.coinbase.to_string(),
            AccountingOperation::Credit => batch.proposer.to_string(),
        };
        self.total_fee += fee;
        self.addresses
            .entry(key)
            .or_insert_with(AddressInfo::new)
            .add_batch(fee, batch);

        Ok(())
    }
}

pub enum AccountingOperation {
    Debit,  // we propose batches for other teams
    Credit, // other teams propose batches for us
}

fn wei_to_eth_string(wei: &U256) -> String {
    let wei_str = wei.to_string();
    let len = wei_str.len();

    if len <= 18 {
        // Pad with leading zeros for fractional part
        let mut s = "0.".to_string();
        s.push_str(&"0".repeat(18 - len));
        s.push_str(&wei_str);
        s.push_str(" ETH");
        s
    } else {
        // Split into integer and fractional parts
        let (int_part, frac_part) = wei_str.split_at(len - 18);
        format!("{int_part}.{frac_part} ETH")
    }
}

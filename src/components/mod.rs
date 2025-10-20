use anyhow::Context;
use anyhow::anyhow;
use ckb_jsonrpc_types::HeaderView;
use ckb_jsonrpc_types::PoolTransactionReject;
use ckb_sdk::CkbRpcClient;
use cursive::Cursive;

pub mod dashboard;

pub trait UpdateToView {
    fn update_to_view(&self, siv: &mut Cursive);
}

pub trait DashboardData: UpdateToView {
    fn fetch_data_through_client(
        &mut self,
        client: &CkbRpcClient,
    ) -> anyhow::Result<Box<dyn DashboardData + Send + Sync>>;
    fn should_update(&self) -> bool {
        true
    }
}

pub trait DashboardState: Sized + Clone + UpdateToView {
    fn update_state(&mut self) -> anyhow::Result<()>;
}

pub fn extract_epoch(epoch_field: u64) -> (u64, u64, u64) {
    let epoch = epoch_field & 0xffffff;
    let epoch_block = (epoch_field >> 24) & 0xffff;
    let epoch_block_count = (epoch_field >> 40) & 0xffff;
    (epoch, epoch_block, epoch_block_count)
}

fn get_average_block_time_and_estimated_epoch_time(
    tip_header: &HeaderView,
    client: &CkbRpcClient,
) -> anyhow::Result<(f64, f64)> {
    let (_, epoch_block, epoch_block_count) = extract_epoch(tip_header.inner.epoch.value());

    let first_block_in_epoch = client
        .get_header_by_number((tip_header.inner.number.value() - epoch_block).into())
        .with_context(|| anyhow!("Unable to get first block header in rpoch"))?
        .unwrap();
    let time_diff_in_epoch =
        tip_header.inner.timestamp.value() - first_block_in_epoch.inner.timestamp.value();
    let average_block_time = time_diff_in_epoch as f64 / 1000.0 / epoch_block as f64;
    let estimated_epoch_time = (epoch_block_count - epoch_block) as f64 * average_block_time;
    Ok((average_block_time, estimated_epoch_time))
}

fn map_pool_transaction_to_reason(rej: &PoolTransactionReject) -> &'static str {
    match rej {
        PoolTransactionReject::LowFeeRate(_) => "Transaction fee lower than config",
        PoolTransactionReject::ExceededMaximumAncestorsCount(_) => {
            "Transaction exceeded maximum ancestors count limit"
        }
        PoolTransactionReject::ExceededTransactionSizeLimit(_) => {
            "Transaction exceeded maximum size limit"
        }
        PoolTransactionReject::Full(_) => "Transaction are replaced because the pool is full",
        PoolTransactionReject::Duplicated(_) => "Transaction already exists in transaction_pool",
        PoolTransactionReject::Malformed(_) => "Malformed transaction",
        PoolTransactionReject::DeclaredWrongCycles(_) => "Declared wrong cycles",
        PoolTransactionReject::Resolve(_) => "Resolve failed",
        PoolTransactionReject::Verification(_) => "Verification failed",
        PoolTransactionReject::Expiry(_) => "Transaction expired",
        PoolTransactionReject::RBFRejected(_) => "RBF rejected",
        PoolTransactionReject::Invalidated(_) => "Invalidated rejected",
    }
}

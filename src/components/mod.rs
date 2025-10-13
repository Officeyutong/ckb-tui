use ckb_sdk::CkbRpcClient;
use cursive::Cursive;

pub mod dashboard;

pub trait UpdateToView {
    fn update_to_view(&self, siv: &mut Cursive);
}

pub trait DashboardData: Sized + UpdateToView {
    fn fetch_data_through_client(client: &CkbRpcClient) -> anyhow::Result<Self>;
    fn should_update() -> bool {
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

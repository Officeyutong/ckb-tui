use ckb_sdk::CkbRpcClient;
use cursive::Cursive;

pub mod dashboard;

pub trait UpdateToView {
    fn update_to_view(&self, siv: &mut Cursive);
}

pub trait FetchData: Sized {
    fn fetch_data_through_client(client: &CkbRpcClient) -> anyhow::Result<Self>;
}

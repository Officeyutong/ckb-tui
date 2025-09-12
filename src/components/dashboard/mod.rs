pub mod overview;

use anyhow::{Context, anyhow};
use cursive::{
    Cursive,
    view::{IntoBoxedView, Nameable, Resizable},
    views::{Button, LinearLayout, Panel, TextView},
};

use crate::components::{
    FetchData, UpdateToView,
    dashboard::{
        names::{REFRESHING_LABEL, TITLE},
        overview::basic_info_dashboard,
    },
};

mod names {
    pub const TITLE: &str = "dashboard_title";
    pub const REFRESHING_LABEL: &str = "dashboard_refreshing_label";
}

pub struct GeneralDashboardData {
    pub network_name: String,
    pub version: String,
}
impl UpdateToView for GeneralDashboardData {
    fn update_to_view(&self, siv: &mut cursive::Cursive) {
        siv.call_on_name(names::TITLE, |view: &mut TextView| {
            view.set_content(format!(
                "{} CKB Node Monitor {}",
                self.network_name, self.version
            ));
        });
    }
}

impl FetchData for GeneralDashboardData {
    fn fetch_data_through_client(client: &ckb_sdk::CkbRpcClient) -> anyhow::Result<Self> {
        let block_chain_info = client
            .get_blockchain_info()
            .with_context(|| anyhow!("Unable to get block chain info"))?;

        Ok(Self {
            network_name: match block_chain_info.chain.as_str() {
                "ckb" => "[Meepo Mainnet]".to_string(),
                "ckb_testnet" => "[Mirana Testnet]".to_string(),
                s => format!("[{}]", s),
            },
            version: format!("unknown version"),
        })
    }
}

pub fn dashboard() -> impl IntoBoxedView + use<> {
    Panel::new(
        LinearLayout::vertical()
            .child(TextView::new("CKB Node Monitor").center().with_name(TITLE))
            .child(TextView::new(" ").center().with_name(REFRESHING_LABEL))
            .child(
                LinearLayout::horizontal()
                    .child(Button::new("Overview", |_| ()).fixed_width(15))
                    .child(Button::new("Blockchain", |_| ()).fixed_width(15))
                    .child(Button::new("Mempool", |_| ()).fixed_width(15))
                    .child(Button::new("Peers", |_| ()).fixed_width(15))
                    .child(Button::new("System Info", |_| ()).fixed_width(15))
                    .child(Button::new("Logs", |_| ()).fixed_width(15)),
            )
            .child(basic_info_dashboard())
            .child(Panel::new(TextView::new(
                "Press [Q] to quit, [Tab] to switch panels, [R] to refresh",
            ))),
    )
}

pub fn set_loading(siv: &mut Cursive, loading: bool) {
    siv.call_on_name(REFRESHING_LABEL, move |view: &mut TextView| {
        if loading {
            view.set_content("Refreshing...");
        } else {
            view.set_content(" ");
        }
    });
}

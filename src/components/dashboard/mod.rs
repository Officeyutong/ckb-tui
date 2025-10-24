pub mod blockchain;
pub mod logs;
pub mod mempool;
pub mod overview;
pub mod peers;

use std::sync::mpsc;

use anyhow::{Context, anyhow};
use ckb_sdk::CkbRpcClient;
use cursive::{
    Cursive,
    view::{IntoBoxedView, Nameable, Resizable},
    views::{Button, LinearLayout, Panel, TextView},
};
use cursive_aligned_view::Alignable;

use crate::{
    components::{
        dashboard::{
            blockchain::blockchain_dashboard,
            logs::{logs_dashboard, FilterLogOption},
            mempool::mempool_dashboard,
            names::{MAIN_LAYOUT, REFRESHING_LABEL, TITLE},
            overview::basic_info_dashboard,
            peers::peers,
        }, DashboardData, UpdateToView
    }, declare_names, CURRENT_TAB
};

declare_names!(names, "dashboard_", TITLE, REFRESHING_LABEL, MAIN_LAYOUT);
#[derive(Clone, Default)]
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

impl DashboardData for GeneralDashboardData {
    fn fetch_data_through_client(
        &mut self,
        client: &CkbRpcClient,
    ) -> anyhow::Result<Box<dyn DashboardData + Send + Sync>> {
        let block_chain_info = client
            .get_blockchain_info()
            .with_context(|| anyhow!("Unable to get block chain info"))?;

        *self = Self {
            network_name: match block_chain_info.chain.as_str() {
                "ckb" => "[Meepo Mainnet]".to_string(),
                "ckb_testnet" => "[Mirana Testnet]".to_string(),
                s => format!("[{}]", s),
            },
            version: format!("unknown version"),
        };

        Ok(Box::new(self.clone()))
    }
}

pub fn dashboard(event_sender: mpsc::Sender<TUIEvent>) -> impl IntoBoxedView + use<> {
    let event_sender_1 = event_sender.clone();
    let event_sender_2 = event_sender.clone();
    let event_sender_3 = event_sender.clone();
    let event_sender_4 = event_sender.clone();
    let event_sender_5 = event_sender.clone();

    Panel::new(
        LinearLayout::vertical()
            .child(TextView::new("CKB Node Monitor").center().with_name(TITLE))
            .child(TextView::new(" ").center().with_name(REFRESHING_LABEL))
            .child(
                LinearLayout::horizontal()
                    .child(
                        Button::new("Overview", move |s| {
                            switch_panel(s, basic_info_dashboard(event_sender_1.clone()), 0)
                        })
                        .fixed_width(15),
                    )
                    .child(
                        Button::new("Blockchain", move |s| {
                            switch_panel(s, blockchain_dashboard(event_sender_2.clone()), 1)
                        })
                        .fixed_width(15),
                    )
                    .child(
                        Button::new("Mempool", move |s| {
                            switch_panel(s, mempool_dashboard(event_sender_3.clone()), 2)
                        })
                        .fixed_width(15),
                    )
                    .child(
                        Button::new("Peers", move |s| {
                            switch_panel(s, peers(event_sender_4.clone()), 3)
                        })
                        .fixed_width(15),
                    )
                    .child(
                        Button::new("Logs", move |s| {
                            switch_panel(s, logs_dashboard(event_sender_5.clone()), 4)
                        })
                        .fixed_width(15),
                    )
                    .align_center(),
            )
            .child(basic_info_dashboard(event_sender.clone()))
            .child(Panel::new(TextView::new(
                "Press [Q] to quit, [Tab] to switch panels, [R] to refresh",
            )))
            .with_name(MAIN_LAYOUT),
    )
}

fn switch_panel(siv: &mut Cursive, panel: impl IntoBoxedView + 'static, panel_index: usize) {
    siv.call_on_name(MAIN_LAYOUT, move |view: &mut LinearLayout| {
        view.remove_child(3);
        view.insert_child(3, panel);
    });
    CURRENT_TAB.store(panel_index, std::sync::atomic::Ordering::SeqCst);
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
pub enum TUIEvent {
    FilterLogEvent(FilterLogOption),
}

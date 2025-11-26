pub mod blockchain;
pub mod logs;
pub mod mempool;
pub mod overview;
pub mod peers;

use std::sync::mpsc;

use anyhow::{Context, anyhow};
use ckb_jsonrpc_types_new::Overview;
use ckb_sdk::CkbRpcClient;
use cursive::{
    Cursive,
    view::{IntoBoxedView, Nameable, Resizable},
    views::{LinearLayout, Panel, RadioGroup, TextView},
};
use cursive_aligned_view::Alignable;
use cursive_spinner_view::SpinnerView;

use crate::{
    CURRENT_TAB,
    components::{
        DashboardData, UpdateToView,
        dashboard::{
            blockchain::blockchain_dashboard,
            logs::{FilterLogOption, logs_dashboard},
            mempool::mempool_dashboard,
            names::{MAIN_LAYOUT, REFRESHING_SPINNER, TITLE},
            overview::basic_info_dashboard,
            peers::peers_dashboard,
        },
    },
    declare_names,
};

declare_names!(names, "dashboard_", TITLE, REFRESHING_SPINNER, MAIN_LAYOUT);
#[derive(Clone, Default)]
pub struct GeneralDashboardData {
    pub network_name: String,
    pub version: Option<String>,
    enable_fetch_overview_data: bool,
}
impl UpdateToView for GeneralDashboardData {
    fn update_to_view(&self, siv: &mut cursive::Cursive) {
        siv.call_on_name(names::TITLE, |view: &mut TextView| {
            view.set_content(format!(
                "{} CKB Node Monitor {}",
                self.network_name,
                match &self.version {
                    Some(v) => v,
                    None => "<unknown version>",
                }
            ));
        });
    }
}

impl DashboardData for GeneralDashboardData {
    fn fetch_data_through_client(
        &mut self,
        client: &CkbRpcClient,
    ) -> anyhow::Result<Box<dyn DashboardData + Send + Sync>> {
        log::debug!("Updating: GeneralDashboardData");
        let block_chain_info = client
            .get_blockchain_info()
            .with_context(|| anyhow!("Unable to get block chain info"))?;
        let version = if self.enable_fetch_overview_data {
            let overview_info: Overview = client
                .post("get_overview", ())
                .with_context(|| anyhow!("Unable to get overview info"))?;
            Some(overview_info.version)
        } else {
            None
        };
        *self = Self {
            network_name: match block_chain_info.chain.as_str() {
                "ckb" => "[Meepo Mainnet]".to_string(),
                "ckb_testnet" => "[Mirana Testnet]".to_string(),
                s => format!("[{}]", s),
            },
            version,
            enable_fetch_overview_data: self.enable_fetch_overview_data,
        };
        log::debug!("Updated: GeneralDashboardData");
        Ok(Box::new(self.clone()))
    }

    fn set_enable_overview_data(&mut self, flag: bool) {
        self.enable_fetch_overview_data = flag;
    }
}

pub fn dashboard(
    event_sender: mpsc::Sender<TUIEvent>,
    cursive: &mut Cursive,
) -> impl IntoBoxedView + use<> {
    let event_sender_0 = event_sender.clone();
    let event_sender_1 = event_sender.clone();
    let event_sender_2 = event_sender.clone();
    let event_sender_3 = event_sender.clone();
    let event_sender_4 = event_sender.clone();
    let mut tab_selector = RadioGroup::<usize>::new().on_change(move |siv, value: &usize| {
        match value {
            idx @ 0 => switch_panel(siv, basic_info_dashboard(event_sender_0.clone()), *idx),
            idx @ 1 => switch_panel(siv, blockchain_dashboard(event_sender_1.clone()), *idx),
            idx @ 2 => switch_panel(siv, mempool_dashboard(event_sender_2.clone()), *idx),
            idx @ 3 => switch_panel(siv, peers_dashboard(event_sender_3.clone()), *idx),
            idx @ 4 => switch_panel(siv, logs_dashboard(event_sender_4.clone()), *idx),
            _ => unreachable!(),
        };
    });

    Panel::new(
        LinearLayout::vertical()
            .child(TextView::new("CKB Node Monitor").center().with_name(TITLE))
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new(" "))
                    .child(
                        SpinnerView::new(cursive.cb_sink().clone()).with_name(REFRESHING_SPINNER),
                    )
                    .align_center(),
            )
            .child(
                LinearLayout::horizontal()
                    .child(tab_selector.button(0, "Overview").fixed_width(15))
                    .child(tab_selector.button(1, "Blockchain").fixed_width(17))
                    .child(tab_selector.button(2, "Mempool").fixed_width(15))
                    .child(tab_selector.button(3, "Peers").fixed_width(15))
                    .child(tab_selector.button(4, "Logs").fixed_width(15))
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
    siv.call_on_name(REFRESHING_SPINNER, move |view: &mut SpinnerView| {
        if loading {
            view.spin_up();
        } else {
            view.spin_down();
        }
    });
}
pub enum TUIEvent {
    FilterLogEvent(FilterLogOption),
    OpenConsensusModal(cursive::CbSink),
}

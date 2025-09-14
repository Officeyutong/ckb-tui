use anyhow::{Context, anyhow};
use ckb_sdk::CkbRpcClient;
use cursive::{
    Cursive,
    view::{IntoBoxedView, Nameable, Resizable, Scrollable},
    views::{LinearLayout, NamedView, Panel, ProgressBar, TextView},
};
use sysinfo::System;

use crate::{
    components::{
        FetchData, UpdateState, UpdateToView,
        dashboard::overview::names::{
            AVERAGE_BLOCK_TIME, AVERAGE_LATENCY, COMMITING_TX, CONNECTED_PEERS, CPU, CPU_HISTORY,
            CURRENT_BLOCK, DIFFICULTY, DISK, EPOCH, ESTIMATED_EPOCH_TIME, ESTIMATED_TIME_LEFT,
            HASH_RATE, PENDING_TX, PROPOSED_TX, RAM, REJECTED_TX, TOTAL_POOL_SIZE,
        },
    },
    utils::bar_chart::SimpleBarChart,
};

mod names {
    pub const CURRENT_BLOCK: &str = "overview_dashboard_current_block";
    pub const SYNCING_PROGRESS: &str = "overview_dashboard_syncing_progress";
    pub const ESTIMATED_TIME_LEFT: &str = "overview_dashboard_estimated_time_left";
    pub const CONNECTED_PEERS: &str = "overview_dashboard_connected_peers";
    pub const AVERAGE_LATENCY: &str = "overview_dashboard_average_latency";
    pub const CPU: &str = "overview_dashboard_cpu";
    pub const RAM: &str = "overview_dashboard_ram";
    pub const DISK: &str = "overview_dashboard_disk";
    pub const EPOCH: &str = "overview_dashboard_epoch";
    pub const ESTIMATED_EPOCH_TIME: &str = "overview_dashboard_estimated_epoch_time";
    pub const AVERAGE_BLOCK_TIME: &str = "overview_dashboard_average_block_time";
    pub const DIFFICULTY: &str = "overview_dashboard_difficulty";
    pub const HASH_RATE: &str = "overview_dashboard_hash_rate";
    pub const TOTAL_POOL_SIZE: &str = "overview_dashboard_total_pool_size";
    pub const PENDING_TX: &str = "overview_dashboard_pending_tx";
    pub const PROPOSED_TX: &str = "overview_dashboard_proposed_tx";
    pub const COMMITING_TX: &str = "overview_dashboard_commiting_tx";
    pub const REJECTED_TX: &str = "overview_dashboard_rejected_tx";
    pub const CPU_HISTORY: &str = "overview_dashboard_cpu_history";
}
#[derive(Default, Clone)]
pub struct OverviewDashboardState {
    pub cpu_history: queue::Queue<f64>,
}

impl UpdateState for OverviewDashboardState {
    fn update_state(mut self) -> Self {
        let mut system = System::new_all();
        system.refresh_cpu_usage();
        self.cpu_history
            .queue(system.global_cpu_usage() as f64 / 100.0)
            .unwrap();
        if self.cpu_history.len() > 20 {
            self.cpu_history.dequeue();
        }
        self
    }
}

impl UpdateToView for OverviewDashboardState {
    fn update_to_view(&self, siv: &mut Cursive) {
        siv.call_on_name(CPU_HISTORY, |view: &mut SimpleBarChart| {
            view.set_data(self.cpu_history.vec()).unwrap();
        });
    }
}

#[derive(Debug, Clone)]
pub struct OverviewDashboardData {
    pub current_block: u64,
    pub total_block: u64,
    // 0~1
    pub syncing_progress: f64,
    // In seconds
    pub estimated_time_left: u64,

    pub inbound_peers: usize,
    pub outbound_peers: usize,
    pub average_latency: usize,

    pub cpu_percent: f64,
    pub ram_total: u64,
    pub ram_used: u64,
    pub disk_used: u64,
    pub disk_total: u64,

    pub tx_pending: u64,
    pub tx_proposed: u64,
    pub tx_commiting: u64,
    pub tx_rejected: u64,
    // in Bytes
    pub total_pool_size: u64,

    pub epoch: u64,
    pub epoch_block: u64,
    pub epoch_block_count: u64,

    // In seconds
    pub estimated_epoch_time: u64,

    // In seconds
    pub average_block_time: u64,

    pub difficulty: f64,
    pub hash_rate: u64,
}

impl UpdateToView for OverviewDashboardData {
    fn update_to_view(&self, siv: &mut Cursive) {
        siv.call_on_name(names::SYNCING_PROGRESS, |view: &mut ProgressBar| {
            view.set_value((self.syncing_progress * 100.0) as usize);
        });
        siv.call_on_name(names::CURRENT_BLOCK, |view: &mut TextView| {
            view.set_content(format!("{}/{}", self.current_block, self.total_block));
        });

        siv.call_on_name(names::ESTIMATED_TIME_LEFT, |view: &mut TextView| {
            view.set_content(format!("{}min", self.estimated_time_left.div_ceil(60)));
        });
        siv.call_on_name(names::CONNECTED_PEERS, |view: &mut TextView| {
            view.set_content(format!(
                "{} ({} outbound / {} inbound)",
                self.inbound_peers + self.outbound_peers,
                self.outbound_peers,
                self.inbound_peers
            ));
        });
        siv.call_on_name(names::AVERAGE_LATENCY, |view: &mut TextView| {
            view.set_content(format!("{}ms", self.average_latency));
        });

        siv.call_on_name(names::EPOCH, |view: &mut TextView| {
            view.set_content(format!(
                "{} ({}/{})",
                self.epoch, self.epoch_block, self.epoch_block_count
            ));
        });
        siv.call_on_name(names::ESTIMATED_EPOCH_TIME, |view: &mut TextView| {
            view.set_content(format!("{}min", self.estimated_epoch_time.div_ceil(60)));
        });
        siv.call_on_name(names::AVERAGE_BLOCK_TIME, |view: &mut TextView| {
            view.set_content(format!("{}s", self.average_block_time));
        });

        siv.call_on_name(names::DIFFICULTY, |view: &mut TextView| {
            view.set_content(format!("{:.2} EH", self.difficulty));
        });

        siv.call_on_name(names::HASH_RATE, |view: &mut TextView| {
            view.set_content(format!("{:.2} PH/s", self.hash_rate));
        });

        siv.call_on_name(names::TOTAL_POOL_SIZE, |view: &mut TextView| {
            view.set_content(format!(
                "{} txs ({:.0} MB)",
                self.tx_pending + self.tx_commiting + self.tx_proposed,
                self.total_pool_size as f64 / 1024.0 / 1024.0
            ));
        });
        siv.call_on_name(names::PENDING_TX, |view: &mut TextView| {
            view.set_content(format!("{}", self.tx_pending));
        });
        siv.call_on_name(names::PROPOSED_TX, |view: &mut TextView| {
            view.set_content(format!("{}", self.tx_proposed));
        });
        siv.call_on_name(names::COMMITING_TX, |view: &mut TextView| {
            view.set_content(format!("{}", self.tx_commiting));
        });
        siv.call_on_name(names::REJECTED_TX, |view: &mut TextView| {
            view.set_content(format!("{}", self.tx_rejected));
        });

        siv.call_on_name(names::PENDING_TX, |view: &mut TextView| {
            view.set_content(format!("{}", self.tx_pending));
        });

        siv.call_on_name(names::CPU, |view: &mut TextView| {
            view.set_content(format!("{:.1}%", self.cpu_percent));
        });
        siv.call_on_name(names::RAM, |view: &mut TextView| {
            view.set_content(format!(
                "{:.1}GB / {:.1}GB",
                self.ram_used as f64 / 1024.0 / 1024.0 / 1024.0,
                self.ram_total as f64 / 1024.0 / 1024.0 / 1024.0
            ));
        });
        siv.call_on_name(names::DISK, |view: &mut TextView| {
            view.set_content(format!(
                "{:.0}GB / {:.0}GB",
                self.disk_used as f64 / 1024.0 / 1024.0 / 1024.0,
                self.disk_total as f64 / 1024.0 / 1024.0 / 1024.0
            ));
        });
    }
}
impl FetchData for OverviewDashboardData {
    fn fetch_data_through_client(client: &CkbRpcClient) -> anyhow::Result<Self> {
        let peers = client
            .get_peers()
            .with_context(|| anyhow!("Unable to get peers"))?
            .into_iter()
            .map(|x| x.is_outbound)
            .collect::<Vec<_>>();
        let outbound_peers = peers.iter().filter(|x| **x).count();
        let inbound_peers = peers.len() - outbound_peers;
        let tip_header = client
            .get_tip_header()
            .with_context(|| anyhow!("Unable to get tip header"))?;
        let tx_pool_info = client
            .tx_pool_info()
            .with_context(|| anyhow!("Unable to get tx pool info"))?;
        let fs_stats = fs2::statvfs(std::env::current_exe()?)?;
        let sync_state = client
            .sync_state()
            .with_context(|| anyhow!("Unable to get sync state"))?;

        let epoch_field = tip_header.inner.epoch.value();
        let mut system = System::new_all();
        system.refresh_cpu_usage();
        system.refresh_memory();
        let data = OverviewDashboardData {
            current_block: tip_header.inner.number.value(),
            total_block: sync_state.best_known_block_number.value(),
            syncing_progress: (tip_header.inner.number.value() as f64
                / sync_state.best_known_block_number.value() as f64),
            estimated_time_left: 123,
            average_latency: 123,
            inbound_peers,
            outbound_peers,
            cpu_percent: system.global_cpu_usage() as f64,
            disk_total: fs_stats.total_space(),
            disk_used: (fs_stats.total_space() - fs_stats.free_space()),
            ram_total: system.total_memory(),
            ram_used: system.used_memory(),
            tx_pending: tx_pool_info.pending.value(),
            tx_proposed: tx_pool_info.proposed.value(),
            tx_commiting: 0,
            tx_rejected: 0,
            total_pool_size: tx_pool_info.total_tx_size.value(),
            epoch: epoch_field & 0xffffff,
            epoch_block: (epoch_field >> 24) & 0xffff,
            epoch_block_count: (epoch_field >> 40) & 0xffff,
            estimated_epoch_time: 0,
            average_block_time: 0,
            difficulty: 0.0,
            hash_rate: 0,
        };

        Ok(data)
    }
}

pub fn basic_info_dashboard() -> impl IntoBoxedView + use<> {
    LinearLayout::vertical()
        .child(
            LinearLayout::horizontal()
                .child(
                    Panel::new(
                        LinearLayout::vertical()
                            .child(TextView::new("[Sync Status]"))
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• Progress:").min_width(20))
                                    .child(ProgressBar::new().range(0, 100)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• Current Block:").min_width(20))
                                    .child(TextView::empty().with_name(CURRENT_BLOCK)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• Est. Time Left:").min_width(20))
                                    .child(TextView::empty().with_name(ESTIMATED_TIME_LEFT)),
                            ),
                    )
                    .min_width(50),
                )
                .child(
                    Panel::new(
                        LinearLayout::vertical()
                            .child(TextView::new("[Peers]"))
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• Connection:").min_width(20))
                                    .child(TextView::empty().with_name(CONNECTED_PEERS)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• Avg. Latency:").min_width(20))
                                    .child(TextView::empty().with_name(AVERAGE_LATENCY)),
                            ),
                    )
                    .min_width(50),
                )
                .scrollable(),
        )
        .child(
            Panel::new(
                LinearLayout::vertical()
                    .child(TextView::new("[Blockchain Health]"))
                    .child(
                        LinearLayout::horizontal()
                            .child(TextView::new("• Epoch:").min_width(20))
                            .child(TextView::empty().with_name(EPOCH)),
                    )
                    .child(
                        LinearLayout::horizontal()
                            .child(TextView::new("• Est. Epoch Time:").min_width(20))
                            .child(TextView::empty().with_name(ESTIMATED_EPOCH_TIME)),
                    )
                    .child(
                        LinearLayout::horizontal()
                            .child(TextView::new("• Avg. Block Time:").min_width(20))
                            .child(TextView::empty().with_name(AVERAGE_BLOCK_TIME)),
                    )
                    .child(
                        LinearLayout::horizontal()
                            .child(TextView::new("• Difficulty:").min_width(20))
                            .child(TextView::empty().with_name(DIFFICULTY)),
                    )
                    .child(
                        LinearLayout::horizontal()
                            .child(TextView::new("• Hash Rate:").min_width(20))
                            .child(TextView::empty().with_name(HASH_RATE)),
                    ),
            )
            .scrollable(),
        )
        .child(
            LinearLayout::horizontal()
                .child(
                    Panel::new(
                        LinearLayout::vertical()
                            .child(TextView::new("[Mempool Activity]"))
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• Total Pool Size:").min_width(20))
                                    .child(TextView::empty().with_name(TOTAL_POOL_SIZE)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("    🟡 Pending:").min_width(20))
                                    .child(TextView::empty().with_name(PENDING_TX)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("    🔵 Proposed:").min_width(20))
                                    .child(TextView::empty().with_name(PROPOSED_TX)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("    🟢 Committing:").min_width(20))
                                    .child(TextView::empty().with_name(COMMITING_TX)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• Rejected:").min_width(20))
                                    .child(TextView::empty().with_name(REJECTED_TX)),
                            ),
                    )
                    .min_width(50),
                )
                .child(
                    Panel::new(
                        LinearLayout::vertical()
                            .child(TextView::new("[System Info]"))
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• CPU:").min_width(10))
                                    .child(TextView::empty().with_name(CPU)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• RAM:").min_width(10))
                                    .child(TextView::empty().with_name(RAM)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• Disk:").min_width(10))
                                    .child(TextView::empty().with_name(DISK)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• CPU load:").min_width(10))
                                    .child(NamedView::new(
                                        CPU_HISTORY,
                                        SimpleBarChart::new(&[
                                            0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9,
                                        ])
                                        .unwrap(),
                                    )),
                            ),
                    )
                    .min_width(50),
                )
                .scrollable(),
        )
}

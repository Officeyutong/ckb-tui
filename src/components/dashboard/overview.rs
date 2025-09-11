use std::sync::atomic::AtomicU64;

use anyhow::{Context, Result, anyhow};
use ckb_sdk::CkbRpcClient;
use cursive::{
    Cursive,
    view::{IntoBoxedView, Nameable, Resizable, Scrollable},
    views::{DummyView, LinearLayout, Panel, TextView},
};
use rand::Rng;
use sysinfo::System;

mod names {
    pub const CURRENT_BLOCK: &str = "overview_dashboard_current_block";
    pub const SYNCING_PROGRESS: &str = "overview_dashboard_syncing_progress";
    pub const ESTIMATED_TIME_LEFT: &str = "overview_dashboard_estimated_time_left";
    pub const CONNECTED_PEERS: &str = "overview_dashboard_connected_peers";
    pub const AVERAGE_LATENCY: &str = "overview_dashboard_average_latency";
    pub const TOTAL_NODES: &str = "overview_dashboard_total_peers";
    pub const RELAYING_TO_COUNT: &str = "overview_dashboard_relaying_to_count";
    pub const CPU: &str = "overview_dashboard_cpu";
    pub const RAM: &str = "overview_dashboard_ram";
    pub const DISK: &str = "overview_dashboard_disk";
    pub const PENDING_TX: &str = "overview_dashboard_pending_tx";
    pub const LAST_COMING_TX: &str = "overview_dashboard_last_coming_tx";
}

#[derive(Debug)]
pub struct OverviewDashboardState {
    pub last_coming_tx: AtomicU64,
}

#[derive(Debug, Clone)]
pub struct OverviewDashboardData {
    pub current_block: usize,
    pub syncing_progress: f64,
    pub estimated_time_left: usize,
    pub inbound_peers: usize,
    pub outbound_peers: usize,
    pub average_latency: usize,
    pub total_nodes_online: usize,
    pub relaying_count: usize,
    pub cpu_percent: f64,
    pub ram_total: usize,
    pub ram_used: usize,
    pub disk_used: usize,
    pub disk_total: usize,
    pub tx_pool_pending: usize,
    pub last_coming_tx: usize,
}

impl OverviewDashboardData {
    pub fn update_to_view(&self, siv: &mut Cursive) {
        siv.call_on_name(names::CURRENT_BLOCK, |view: &mut TextView| {
            view.set_content(format!("  • Current Block: #{}", self.current_block));
        });
        siv.call_on_name(names::SYNCING_PROGRESS, |view: &mut TextView| {
            view.set_content(format!(
                "  • Syncing: {:.02}%",
                self.syncing_progress * 100.0
            ));
        });
        siv.call_on_name(names::ESTIMATED_TIME_LEFT, |view: &mut TextView| {
            view.set_content(format!(
                "  • Estimated time left: {}min",
                self.estimated_time_left.div_ceil(60)
            ));
        });
        siv.call_on_name(names::CONNECTED_PEERS, |view: &mut TextView| {
            view.set_content(format!(
                "  • Connected {} ({} outbound / {} inbound)",
                self.inbound_peers + self.outbound_peers,
                self.outbound_peers,
                self.inbound_peers
            ));
        });
        siv.call_on_name(names::AVERAGE_LATENCY, |view: &mut TextView| {
            view.set_content(format!("  • Avg latency: {}ms", self.average_latency));
        });
        siv.call_on_name(names::TOTAL_NODES, |view: &mut TextView| {
            view.set_content(format!(
                "  • You're 1 of ~{} full nodes online",
                self.total_nodes_online
            ));
        });
        siv.call_on_name(names::RELAYING_TO_COUNT, |view: &mut TextView| {
            view.set_content(format!(
                "  • Currently relaying data to {} peers",
                self.relaying_count
            ));
        });
        siv.call_on_name(names::CPU, |view: &mut TextView| {
            view.set_content(format!("  • CPU: {:.1}%", self.cpu_percent));
        });
        siv.call_on_name(names::RAM, |view: &mut TextView| {
            view.set_content(format!(
                "  • RAM: {:.1}GB / {:.1}GB",
                self.ram_used as f64 / 1024.0,
                self.ram_total as f64 / 1024.0
            ));
        });
        siv.call_on_name(names::DISK, |view: &mut TextView| {
            view.set_content(format!(
                "  • Disk: {}GB / {}GB",
                self.disk_used, self.disk_total
            ));
        });
        siv.call_on_name(names::PENDING_TX, |view: &mut TextView| {
            view.set_content(format!("  • Pending: {} txs", self.tx_pool_pending));
        });
        siv.call_on_name(names::LAST_COMING_TX, |view: &mut TextView| {
            view.set_content(format!(
                "  • Last incoming tx: {}s ago",
                self.last_coming_tx
            ));
        });
    }
}

pub fn fetch_overview_data(client: &CkbRpcClient) -> Result<OverviewDashboardData> {
    let mut rng = rand::rng();
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

    let mut system = System::new_all();
    system.refresh_cpu_usage();
    system.refresh_memory();
    let data = OverviewDashboardData {
        average_latency: rng.random_range(1..1000),
        current_block: tip_header.inner.number.value() as usize,
        estimated_time_left: rng.random_range(1..1000),
        inbound_peers,
        outbound_peers,
        syncing_progress: rng.random(),
        cpu_percent: system.global_cpu_usage() as f64,
        disk_total: fs_stats.total_space() as usize / 1024 / 1024 / 1024,
        disk_used: (fs_stats.total_space() - fs_stats.free_space()) as usize / 1024 / 1024 / 1024,
        last_coming_tx: rng.random_range(1..1000),
        ram_total: system.total_memory() as usize / 1024 / 1024,
        ram_used: system.used_memory() as usize / 1024 / 1024,
        relaying_count: rng.random_range(1..1000),
        total_nodes_online: rng.random_range(1..1000),
        tx_pool_pending: tx_pool_info.pending.value() as usize,
    };

    Ok(data)
}
pub fn basic_info_dashboard() -> impl IntoBoxedView + use<> {
    LinearLayout::vertical()
        .child(
            Panel::new(
                LinearLayout::vertical()
                    .child(TextView::new("[Sync Status]"))
                    .child(
                        TextView::new("  • Current Block: Loading..").with_name(names::CURRENT_BLOCK),
                    )
                    .child(TextView::new("  • Syncing: Loading..").with_name(names::SYNCING_PROGRESS))
                    .child(
                        TextView::new("  • Estimated time left: Loading..")
                            .with_name(names::ESTIMATED_TIME_LEFT),
                    )
                    .child(DummyView::new().fixed_height(1))
                    .child(TextView::new("[Peers]"))
                    .child(
                        TextView::new("  • Connected: Loading..")
                            .with_name(names::CONNECTED_PEERS),
                    )
                    .child(
                        TextView::new("  • Avg latency: Loading..").with_name(names::AVERAGE_LATENCY),
                    ),
            )
            .scrollable(),
        )
        .child(
            Panel::new(
                LinearLayout::vertical()
                    .child(TextView::new("[Network Impact]"))
                    .child(
                        TextView::new("")
                            .with_name(names::TOTAL_NODES),
                    )
                    .child(
                        TextView::new("")
                            .with_name(names::RELAYING_TO_COUNT),
                    )
                    .child(TextView::new("  • Your node is reachable ✓")),
            )
            .scrollable(),
        )
        .child(
            LinearLayout::horizontal()
                .child(Panel::new(
                    LinearLayout::vertical()
                        .child(TextView::new("[System Info]"))
                        .child(TextView::new("").with_name(names::CPU))
                        .child(TextView::new("").with_name(names::RAM))
                        .child(TextView::new("").with_name(names::DISK))
                        .min_width(50),
                ))
                .child(Panel::new(
                    LinearLayout::vertical()
                        .child(TextView::new("[Tx Pool]"))
                        .child(TextView::new("").with_name(names::PENDING_TX))
                        .child(
                            TextView::new("")
                                .with_name(names::LAST_COMING_TX),
                        )
                        .min_width(50),
                ))
                .scrollable(),
        )
}

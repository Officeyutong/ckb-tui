use cursive::{
    Cursive,
    view::{IntoBoxedView, Nameable, Resizable, Scrollable},
    views::{DummyView, LinearLayout, Panel, TextView},
};

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
    pub fn update_to_view(&self, _prefix: &str, siv: &mut Cursive) {
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

pub fn basic_info_dashboard() -> impl IntoBoxedView + use<> {
    LinearLayout::vertical()
        .child(
            Panel::new(
                LinearLayout::vertical()
                    .child(TextView::new("[Sync Status]"))
                    .child(
                        TextView::new("  • Current Block: #123456").with_name(names::CURRENT_BLOCK),
                    )
                    .child(TextView::new("  • Syncing: 87.5%").with_name(names::SYNCING_PROGRESS))
                    .child(
                        TextView::new("  • Estimated time left: ~12min")
                            .with_name(names::ESTIMATED_TIME_LEFT),
                    )
                    .child(DummyView::new().fixed_height(1))
                    .child(TextView::new("[Peers]"))
                    .child(
                        TextView::new("  • Connected: 8 (6 outbound / 2 in bound)")
                            .with_name(names::CONNECTED_PEERS),
                    )
                    .child(
                        TextView::new("  • Avg latency: 54ms").with_name(names::AVERAGE_LATENCY),
                    ),
            )
            .scrollable(),
        )
        .child(
            Panel::new(
                LinearLayout::vertical()
                    .child(TextView::new("[Network Impact]"))
                    .child(
                        TextView::new("  • You're 1 of ~950 full nodes online")
                            .with_name(names::TOTAL_NODES),
                    )
                    .child(
                        TextView::new("  • Currently relaying data to 6 peers")
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
                        .child(TextView::new("  • CPU: 14%").with_name(names::CPU))
                        .child(TextView::new("  • RAM: 2.3GB / 8GB").with_name(names::RAM))
                        .child(TextView::new("  • Disk: 111GB / 222GB").with_name(names::DISK))
                        .min_width(50),
                ))
                .child(Panel::new(
                    LinearLayout::vertical()
                        .child(TextView::new("[Tx Pool]"))
                        .child(TextView::new("  • Pending: 120 txs").with_name(names::PENDING_TX))
                        .child(
                            TextView::new("  • Last incoming tx: 15s ago")
                                .with_name(names::LAST_COMING_TX),
                        )
                        .min_width(50),
                ))
                .scrollable(),
        )
        .child(Panel::new(TextView::new(
            "Press [Q] to quit, [Tab] to switch panels, [R] to refresh",
        )))
}

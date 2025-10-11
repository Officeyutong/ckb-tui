use anyhow::{Context, anyhow};
use chrono::Local;
use ckb_sdk::CkbRpcClient;
use cursive::{
    Cursive,
    view::{IntoBoxedView, Nameable, Resizable, Scrollable},
    views::{LinearLayout, NamedView, Panel, ProgressBar, TextView},
};
use sysinfo::{Disks, Networks, System};

use crate::{
    CURRENT_TAB,
    components::{
        DashboardData, DashboardState, UpdateToView,
        dashboard::overview::names::{
            AVERAGE_BLOCK_TIME, AVERAGE_FEE_RATE, AVERAGE_LATENCY, COMMITING_TX, CONNECTED_PEERS,
            CPU, CPU_HISTORY, CURRENT_BLOCK, DIFFICULTY, DISK_SPEED, DISK_USAGE, EPOCH,
            ESTIMATED_EPOCH_TIME, ESTIMATED_TIME_LEFT, HASH_RATE, NETWORK, PENDING_TX, PROPOSED_TX,
            RAM, REJECTED_TX, SYNCING_PROGRESS, TOTAL_POOL_SIZE,
        },
        extract_epoch,
    },
    declare_names, update_text,
    utils::bar_chart::SimpleBarChart,
};

declare_names!(
    names,
    "overview_dashboard_",
    CURRENT_BLOCK,
    SYNCING_PROGRESS,
    ESTIMATED_TIME_LEFT,
    CONNECTED_PEERS,
    AVERAGE_LATENCY,
    CPU,
    RAM,
    DISK_USAGE,
    EPOCH,
    ESTIMATED_EPOCH_TIME,
    AVERAGE_BLOCK_TIME,
    DIFFICULTY,
    HASH_RATE,
    TOTAL_POOL_SIZE,
    PENDING_TX,
    PROPOSED_TX,
    COMMITING_TX,
    REJECTED_TX,
    CPU_HISTORY,
    DISK_SPEED,
    AVERAGE_FEE_RATE,
    NETWORK
);
#[derive(Clone)]
pub struct OverviewDashboardState {
    pub cpu_history: queue::Queue<f64>,
    pub last_update: chrono::DateTime<Local>,
    pub total_disk_write_bytes: u64,
    pub total_disk_read_bytes: u64,
    // Bytes per sec
    pub disk_write_speed: f64,
    // Bytes per sec
    pub disk_read_speed: f64,

    pub total_network_send_bytes: u64,
    pub total_network_receive_bytes: u64,
    // Bytes per sec
    pub network_send_speed: f64,
    // Bytes per sec
    pub network_receive_speed: f64,

    pub client: CkbRpcClient,
    pub current_block: u64,
    pub total_block: u64,
    // In seconds
    pub estimated_time_left: u64,

    pub epoch: u64,
    pub epoch_block: u64,
    pub epoch_block_count: u64,

    pub estimated_epoch_time: f64,
    pub average_block_time: f64,
}

impl OverviewDashboardState {
    fn get_total_read_and_total_write_bytes_for_disk() -> (u64, u64) {
        let disks = Disks::new_with_refreshed_list();
        let (read, write) = disks
            .into_iter()
            .map(|x| x.usage())
            .map(|x| (x.total_read_bytes, x.total_written_bytes))
            .fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));

        (read, write)
    }

    fn get_total_send_and_receive_bytes_for_network_devices() -> (u64, u64) {
        let networks = Networks::new_with_refreshed_list();
        let (send, received) = networks
            .into_iter()
            .map(|x| x.1)
            .map(|x| (x.total_transmitted(), x.total_received()))
            .fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));

        (send, received)
    }
    pub fn new(client: CkbRpcClient) -> Self {
        let (read, write) = Self::get_total_read_and_total_write_bytes_for_disk();
        let (send, receive) = Self::get_total_send_and_receive_bytes_for_network_devices();

        Self {
            cpu_history: Default::default(),
            last_update: chrono::Local::now(),
            disk_read_speed: 1.0,
            disk_write_speed: 1.0,
            total_disk_read_bytes: read,
            total_disk_write_bytes: write,
            network_receive_speed: 1.0,
            network_send_speed: 1.0,
            total_network_receive_bytes: receive,
            total_network_send_bytes: send,
            client,
            current_block: 0,
            estimated_time_left: 100,
            total_block: 1,
            epoch: 0,
            epoch_block: 0,
            epoch_block_count: 1,
            average_block_time: -1.0,
            estimated_epoch_time: -1.0,
        }
    }
}

impl DashboardState for OverviewDashboardState {
    fn update_state(&mut self) -> anyhow::Result<()> {
        let mut system = System::new_all();
        system.refresh_cpu_usage();
        self.cpu_history
            .queue(system.global_cpu_usage() as f64 / 100.0)
            .unwrap();
        if self.cpu_history.len() > 20 {
            self.cpu_history.dequeue();
        }
        let now = chrono::Local::now();
        let diff_secs = ((now - self.last_update).num_milliseconds() as f64) / 1e3;

        {
            let (read, write) = Self::get_total_read_and_total_write_bytes_for_disk();
            self.disk_read_speed = (read - self.total_disk_read_bytes) as f64 / diff_secs;
            self.disk_write_speed = (write - self.total_disk_write_bytes) as f64 / diff_secs;
            self.total_disk_read_bytes = read;
            self.total_disk_write_bytes = write;
        }
        {
            let (send, receive) = Self::get_total_send_and_receive_bytes_for_network_devices();
            self.network_receive_speed =
                (receive - self.total_network_receive_bytes) as f64 / diff_secs;
            self.network_send_speed = (send - self.total_network_send_bytes) as f64 / diff_secs;
            self.total_network_receive_bytes = receive;
            self.total_network_send_bytes = send;
        }
        let tip_header = self
            .client
            .get_tip_header()
            .with_context(|| anyhow!("Unable to get tip header"))?;
        {
            let sync_state = self
                .client
                .sync_state()
                .with_context(|| anyhow!("Unable to get sync_state"))?;
            let current_block = tip_header.inner.number.value();
            let total_block = sync_state.best_known_block_number.value();
            // blocks per sec
            let block_sync_speed = (current_block - self.current_block) as f64 / diff_secs;
            let estimated_seconds = (total_block - current_block) as f64 / block_sync_speed;
            self.current_block = current_block;
            self.total_block = total_block;
            self.estimated_time_left = estimated_seconds.ceil() as u64;
        }
        {
            let epoch_field = tip_header.inner.epoch.value();
            let (epoch, epoch_block, epoch_block_count) = extract_epoch(epoch_field);

            self.epoch = epoch;
            self.epoch_block = epoch_block;
            self.epoch_block_count = epoch_block_count;
        }
        self.last_update = chrono::Local::now();
        Ok(())
    }
}

impl UpdateToView for OverviewDashboardState {
    fn update_to_view(&self, siv: &mut Cursive) {
        siv.call_on_name(CPU_HISTORY, |view: &mut SimpleBarChart| {
            view.set_data(self.cpu_history.vec()).unwrap();
        });
        update_text!(
            siv,
            DISK_SPEED,
            format!(
                "{:.1} MB/s (Read)   {:.1} MB/s (Write)",
                self.disk_read_speed / 1024.0 / 1024.0,
                self.disk_write_speed / 1024.0 / 1024.0
            )
        );
        update_text!(
            siv,
            NETWORK,
            format!(
                "{:.1} MB/s (In)   {:.1} MB/s (Out)",
                self.network_receive_speed / 1024.0 / 1024.0,
                self.network_send_speed / 1024.0 / 1024.0
            )
        );
        siv.call_on_name(names::SYNCING_PROGRESS, |view: &mut ProgressBar| {
            view.set_value(
                ((self.current_block as f64 / self.total_block as f64) * 100.0) as usize,
            );
        });
        update_text!(
            siv,
            names::CURRENT_BLOCK,
            format!("{}/{}", self.current_block, self.total_block)
        );

        update_text!(
            siv,
            names::ESTIMATED_TIME_LEFT,
            format!("{}min", self.estimated_time_left.div_ceil(60))
        );
        update_text!(
            siv,
            names::EPOCH,
            format!(
                "{} ({}/{})",
                self.epoch, self.epoch_block, self.epoch_block_count
            )
        );
        update_text!(
            siv,
            names::ESTIMATED_EPOCH_TIME,
            format!("{}min", (self.estimated_epoch_time / 60.0).ceil())
        );
        update_text!(
            siv,
            names::AVERAGE_BLOCK_TIME,
            format!("{}s", self.average_block_time)
        );
    }
}

#[derive(Debug, Clone)]
pub struct OverviewDashboardData {
    pub inbound_peers: usize,
    pub outbound_peers: usize,
    pub average_latency: isize,

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
    pub total_pool_size: i64,
    pub difficulty: f64,
    pub hash_rate: u64,

    // shannons per KB
    pub average_fee_rate: f64,
}

impl UpdateToView for OverviewDashboardData {
    fn update_to_view(&self, siv: &mut Cursive) {
        update_text!(
            siv,
            names::CONNECTED_PEERS,
            format!(
                "{} ({} outbound / {} inbound)",
                self.inbound_peers + self.outbound_peers,
                self.outbound_peers,
                self.inbound_peers
            )
        );
        update_text!(
            siv,
            names::AVERAGE_LATENCY,
            format!("{}ms", self.average_latency)
        );
        update_text!(siv, names::DIFFICULTY, format!("{:.2} EH", self.difficulty));
        update_text!(siv, names::HASH_RATE, format!("{:.2} PH/s", self.hash_rate));
        update_text!(
            siv,
            names::TOTAL_POOL_SIZE,
            format!(
                "{} txs ({:.0} MB)",
                self.tx_pending + self.tx_commiting + self.tx_proposed,
                self.total_pool_size as f64 / 1024.0 / 1024.0
            )
        );
        update_text!(siv, names::PENDING_TX, format!("{}", self.tx_pending));
        update_text!(siv, names::PROPOSED_TX, format!("{}", self.tx_proposed));
        update_text!(siv, names::COMMITING_TX, format!("{}", self.tx_commiting));
        update_text!(siv, names::REJECTED_TX, format!("{}", self.tx_rejected));
        update_text!(siv, names::CPU, format!("{:.1}%", self.cpu_percent));
        update_text!(
            siv,
            names::RAM,
            format!(
                "{:.1}GB / {:.1}GB",
                self.ram_used as f64 / 1024.0 / 1024.0 / 1024.0,
                self.ram_total as f64 / 1024.0 / 1024.0 / 1024.0
            )
        );
        update_text!(
            siv,
            names::DISK_USAGE,
            format!(
                "{:.0}GB / {:.0}GB ({:.2}%)",
                self.disk_used as f64 / 1024.0 / 1024.0 / 1024.0,
                self.disk_total as f64 / 1024.0 / 1024.0 / 1024.0,
                (self.disk_used as f64 / self.disk_total as f64 * 100.0)
            )
        );
        update_text!(
            siv,
            names::AVERAGE_FEE_RATE,
            format!("{} shannons/KB", self.average_fee_rate)
        );
    }
}
impl DashboardData for OverviewDashboardData {
    fn should_update(&self) -> bool {
        CURRENT_TAB.load(std::sync::atomic::Ordering::SeqCst) == 0
    }
    fn fetch_data_through_client(client: &CkbRpcClient) -> anyhow::Result<Self> {
        let peers = client
            .get_peers()
            .with_context(|| anyhow!("Unable to get peers"))?
            .into_iter()
            .map(|x| x.is_outbound)
            .collect::<Vec<_>>();
        let outbound_peers = peers.iter().filter(|x| **x).count();
        let inbound_peers = peers.len() - outbound_peers;
        let tx_pool_info = client
            .tx_pool_info()
            .with_context(|| anyhow!("Unable to get tx pool info"))?;
        let fs_stats = fs2::statvfs(std::env::current_exe()?)?;
        let fee_rate_statistics = client
            .get_fee_rate_statistics(None)
            .with_context(|| anyhow!("Unable to get fee rate statistics"))?;

        let mut system = System::new_all();
        system.refresh_cpu_usage();
        system.refresh_memory();
        let data = OverviewDashboardData {
            average_latency: -1,
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
            total_pool_size: -1,
            difficulty: -1.0,
            hash_rate: 0,
            average_fee_rate: fee_rate_statistics.unwrap().mean.value() as f64,
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
                                    .child(
                                        ProgressBar::new()
                                            .range(0, 100)
                                            .with_name(SYNCING_PROGRESS)
                                            .min_width(30),
                                    ),
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
                                    .child(TextView::new("• Avg.Fee Rate:").min_width(20))
                                    .child(TextView::empty().with_name(AVERAGE_FEE_RATE)),
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
                                    .child(TextView::new("• CPU:").min_width(12))
                                    .child(TextView::empty().with_name(CPU)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• RAM:").min_width(12))
                                    .child(TextView::empty().with_name(RAM)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• Disk:").min_width(12))
                                    .child(TextView::empty().with_name(DISK_USAGE)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• CPU load:").min_width(12))
                                    .child(NamedView::new(
                                        CPU_HISTORY,
                                        SimpleBarChart::new(&[
                                            0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9,
                                        ])
                                        .unwrap(),
                                    )),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• Disk I/O:").min_width(12))
                                    .child(TextView::empty().with_name(DISK_SPEED)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• Network:").min_width(12))
                                    .child(TextView::empty().with_name(NETWORK)),
                            ),
                    )
                    .min_width(50),
                )
                .scrollable(),
        )
}

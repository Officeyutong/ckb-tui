use std::sync::mpsc;

use anyhow::{Context, anyhow};
use chrono::Local;
use ckb_jsonrpc_types_new::Overview;
use ckb_sdk::CkbRpcClient;
use cursive::{
    Cursive,
    view::{IntoBoxedView, Nameable, Resizable, Scrollable},
    views::{LinearLayout, NamedView, Panel, ProgressBar, TextView},
};
use numext_fixed_uint::{U256, u256};

use crate::{
    components::{
        dashboard::{
            overview::names::{
                AVERAGE_BLOCK_TIME, AVERAGE_FEE_RATE, AVERAGE_LATENCY, COMMITTING_TX,
                CONNECTED_PEERS, CPU, CPU_HISTORY, CURRENT_BLOCK, DIFFICULTY, DISK_SPEED,
                DISK_USAGE, EPOCH, ESTIMATED_EPOCH_TIME, ESTIMATED_TIME_LEFT, HASH_RATE, NETWORK,
                PENDING_TX, PROPOSED_TX, RAM, REJECTED_TX, SYNCING_PROGRESS, TOTAL_POOL_SIZE,
            }, TUIEvent
        }, extract_epoch, get_average_block_time_and_estimated_epoch_time, DashboardData, DashboardState, UpdateToView
    }, declare_names, update_text, utils::{bar_chart::SimpleBarChart, hash_rate_to_string}, CURRENT_TAB
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
    COMMITTING_TX,
    REJECTED_TX,
    CPU_HISTORY,
    DISK_SPEED,
    AVERAGE_FEE_RATE,
    NETWORK
);

#[derive(Clone)]

struct GetOverviewOfOverviewDashboardState {
    pub cpu_history: queue::Queue<f64>,
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
    pub cpu_percent: f64,
    pub ram_total: u64,
    pub ram_used: u64,
    pub disk_used: u64,
    pub disk_total: u64,

    pub difficulty: U256,
    pub hash_rate: f64,
}

#[derive(Clone)]
pub struct OverviewDashboardState {
    pub last_update: chrono::DateTime<Local>,

    pub client: CkbRpcClient,
    pub current_block: u64,
    pub total_block: u64,
    // In seconds
    pub estimated_time_left: u64,

    overview_data: Option<GetOverviewOfOverviewDashboardState>,
}

impl OverviewDashboardState {
    fn get_total_read_and_total_write_bytes_for_disk(overview: &Overview) -> (u64, u64) {
        let disks = &overview.sys.disk_usage;

        (disks.total_read_bytes, disks.total_written_bytes)
    }

    fn get_total_send_and_receive_bytes_for_network_devices(overview: &Overview) -> (u64, u64) {
        let networks = &overview.sys.global.networks;
        let (send, received) = networks
            .into_iter()
            .map(|x| (x.total_transmitted, x.total_received))
            .fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));

        (send, received)
    }

    fn extract_cpu_percent_and_disk_total_and_disk_used_and_ram_total_and_ram_used_from_overview(
        overview_data: &Overview,
    ) -> (f64, u64, u64, u64, u64) {
        let cpu_percent = overview_data.sys.global.global_cpu_usage as f64;
        let (disk_total, disk_used) = overview_data
            .sys
            .global
            .disks
            .iter()
            .map(|x| (x.total_space, x.total_space - x.available_space))
            .fold((0, 0), |v1, v2| (v1.0 + v2.0, v1.1 + v2.1));

        let ram_total = overview_data.sys.global.total_memory;
        let ram_used = overview_data.sys.global.used_memory;

        (cpu_percent, disk_total, disk_used, ram_total, ram_used)
    }

    pub fn new(client: CkbRpcClient, enable_overview_data: bool) -> anyhow::Result<Self> {
        let overview_data = if enable_overview_data {
            let overview = client.post::<(), Overview>("get_overview", ())?;

            let (read, write) = Self::get_total_read_and_total_write_bytes_for_disk(&overview);
            let (send, receive) =
                Self::get_total_send_and_receive_bytes_for_network_devices(&overview);

            let  (cpu_percent, disk_total, disk_used, ram_total, ram_used) = Self::extract_cpu_percent_and_disk_total_and_disk_used_and_ram_total_and_ram_used_from_overview(&overview);
            Some(GetOverviewOfOverviewDashboardState {
                cpu_history: Default::default(),
                disk_read_speed: 1.0,
                disk_write_speed: 1.0,
                total_disk_read_bytes: read,
                total_disk_write_bytes: write,
                network_receive_speed: 1.0,
                network_send_speed: 1.0,
                total_network_receive_bytes: receive,
                total_network_send_bytes: send,
                cpu_percent,
                disk_total,
                disk_used,
                ram_total,
                ram_used,
                difficulty: u256!("0"),
                hash_rate: 0.0,
            })
        } else {
            None
        };
        Ok(Self {
            last_update: chrono::Local::now(),
            overview_data,
            client,
            current_block: 0,
            estimated_time_left: 100,
            total_block: 1,
        })
    }
}

impl DashboardState for OverviewDashboardState {
    fn update_state(&mut self) -> anyhow::Result<()> {
        log::info!("Updating: OverviewDashboardState");
        let now = chrono::Local::now();
        let diff_secs = ((now - self.last_update).num_milliseconds() as f64) / 1e3;
        if let Some(data) = &mut self.overview_data {
            let overview_data = self.client.post::<(), Overview>("get_overview", ())?;

            data.cpu_history
                .queue(overview_data.sys.global.global_cpu_usage as f64 / 100.0)
                .unwrap();
            if data.cpu_history.len() > 20 {
                data.cpu_history.dequeue();
            }
            data.hash_rate = overview_data
                .mining
                .hash_rate
                .to_string()
                .parse::<f64>()
                .unwrap();
            data.difficulty = overview_data.mining.difficulty.clone();

            {
                let (read, write) =
                    Self::get_total_read_and_total_write_bytes_for_disk(&overview_data);
                data.disk_read_speed = (read - data.total_disk_read_bytes) as f64 / diff_secs;
                data.disk_write_speed = (write - data.total_disk_write_bytes) as f64 / diff_secs;
                data.total_disk_read_bytes = read;
                data.total_disk_write_bytes = write;
            }
            {
                let (send, receive) =
                    Self::get_total_send_and_receive_bytes_for_network_devices(&overview_data);
                data.network_receive_speed =
                    (receive - data.total_network_receive_bytes) as f64 / diff_secs;
                data.network_send_speed = (send - data.total_network_send_bytes) as f64 / diff_secs;
                data.total_network_receive_bytes = receive;
                data.total_network_send_bytes = send;
            }
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
            let estimated_seconds = if total_block > current_block {
                (total_block - current_block) as f64 / block_sync_speed
            } else {
                0.0
            };
            self.current_block = current_block;
            self.total_block = total_block;
            self.estimated_time_left = estimated_seconds.ceil() as u64;
        }

        self.last_update = chrono::Local::now();
        log::info!("Updated: OverviewDashboardState");
        Ok(())
    }
}

impl UpdateToView for OverviewDashboardState {
    fn update_to_view(&self, siv: &mut Cursive) {
        if let Some(data) = &self.overview_data {
            siv.call_on_name(CPU_HISTORY, |view: &mut SimpleBarChart| {
                view.set_data(data.cpu_history.vec()).unwrap();
            });
            update_text!(
                siv,
                DISK_SPEED,
                format!(
                    "{:.1} MB/s (Read)   {:.1} MB/s (Write)",
                    data.disk_read_speed / 1024.0 / 1024.0,
                    data.disk_write_speed / 1024.0 / 1024.0
                )
            );
            update_text!(
                siv,
                NETWORK,
                format!(
                    "{:.1} MB/s (In)   {:.1} MB/s (Out)",
                    data.network_receive_speed / 1024.0 / 1024.0,
                    data.network_send_speed / 1024.0 / 1024.0
                )
            );

            update_text!(siv, names::CPU, format!("{:.1}%", data.cpu_percent));
            update_text!(
                siv,
                names::RAM,
                format!(
                    "{:.1}GB / {:.1}GB",
                    data.ram_used as f64 / 1024.0 / 1024.0 / 1024.0,
                    data.ram_total as f64 / 1024.0 / 1024.0 / 1024.0
                )
            );
            update_text!(
                siv,
                names::DISK_USAGE,
                format!(
                    "{:.0}GB / {:.0}GB ({:.2}%)",
                    data.disk_used as f64 / 1024.0 / 1024.0 / 1024.0,
                    data.disk_total as f64 / 1024.0 / 1024.0 / 1024.0,
                    (data.disk_used as f64 / data.disk_total as f64 * 100.0)
                )
            );
            update_text!(siv, names::DIFFICULTY, format!("{:x}", data.difficulty));
            update_text!(
                siv,
                names::HASH_RATE,
                hash_rate_to_string(data.hash_rate)
            );
        } else {
            siv.call_on_name(CPU_HISTORY, |view: &mut SimpleBarChart| {
                view.set_data(&vec![]).unwrap();
            });
            update_text!(siv, DISK_SPEED, "N/A");
            update_text!(siv, NETWORK, "N/A");
            update_text!(siv, names::CPU, "N/A");
            update_text!(siv, names::RAM, "N/A");
            update_text!(siv, names::DISK_USAGE, "N/A");
            update_text!(siv, names::DIFFICULTY, "N/A");
            update_text!(siv, names::HASH_RATE, "N/A");
        };
        siv.call_on_name(names::SYNCING_PROGRESS, |view: &mut ProgressBar| {
            view.set_value(
                (((self.current_block as f64 / self.total_block as f64) * 100.0) as usize).min(100),
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
    }
}

#[derive(Debug, Clone)]
struct GetOverviewOfOverviewDashboardData {
    pub tx_pending: u64,
    pub tx_proposed: u64,
    pub tx_committing: u64,
    pub tx_rejected: u64,
    // in bytes
    pub total_pool_size_in_bytes: u64,
}

#[derive(Debug, Clone, Default)]
pub struct OverviewDashboardData {
    pub inbound_peers: usize,
    pub outbound_peers: usize,
    pub average_latency: isize,
    overview_data: Option<GetOverviewOfOverviewDashboardData>,
    // shannons per KB
    pub average_fee_rate: Option<u64>,

    pub epoch: u64,
    pub epoch_block: u64,
    pub epoch_block_count: u64,

    pub estimated_epoch_time: f64,
    pub average_block_time: f64,

    enable_fetch_overview_data: bool,
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

        if let Some(data) = &self.overview_data {
            update_text!(
                siv,
                names::TOTAL_POOL_SIZE,
                format!(
                    "{} txs ({:.1} MB)",
                    data.tx_committing + data.tx_pending + data.tx_proposed,
                    data.total_pool_size_in_bytes as f64 / 1024.0 / 1024.0
                )
            );
            update_text!(siv, names::PENDING_TX, format!("{}", data.tx_pending));
            update_text!(siv, names::PROPOSED_TX, format!("{}", data.tx_proposed));
            update_text!(siv, names::COMMITTING_TX, format!("{}", data.tx_committing));
            update_text!(siv, names::REJECTED_TX, format!("{}", data.tx_rejected));
        } else {
            update_text!(siv, names::TOTAL_POOL_SIZE, format!("N/A"));
            update_text!(siv, names::PENDING_TX, format!("N/A"));
            update_text!(siv, names::PROPOSED_TX, format!("N/A"));
            update_text!(siv, names::COMMITTING_TX, format!("N/A"));
            update_text!(siv, names::REJECTED_TX, format!("N/A"));
        };

        update_text!(
            siv,
            names::AVERAGE_FEE_RATE,
            match self.average_fee_rate {
                None => format!("N/A"),
                Some(v) => format!("{} shannons/KB", v),
            }
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
            format!("{} min", (self.estimated_epoch_time / 60.0).ceil())
        );
        update_text!(
            siv,
            names::AVERAGE_BLOCK_TIME,
            format!("{:.2} s", self.average_block_time)
        );
    }
}
impl DashboardData for OverviewDashboardData {
    fn should_update(&self) -> bool {
        CURRENT_TAB.load(std::sync::atomic::Ordering::SeqCst) == 0
    }
    fn fetch_data_through_client(
        &mut self,
        client: &CkbRpcClient,
    ) -> anyhow::Result<Box<dyn DashboardData + Send + Sync>> {
        log::info!("Updating: OverviewDashboardData");
        let peers = client
            .get_peers()
            .with_context(|| anyhow!("Unable to get peers"))?
            .into_iter()
            .map(|x| x.is_outbound)
            .collect::<Vec<_>>();
        let outbound_peers = peers.iter().filter(|x| **x).count();
        let inbound_peers = peers.len() - outbound_peers;
        let fee_rate_statistics = client
            .get_fee_rate_statistics(None)
            .with_context(|| anyhow!("Unable to get fee rate statistics"))?;

        let tip_header = client
            .get_tip_header()
            .with_context(|| anyhow!("Unable to get tip header"))?;

        let epoch_field = tip_header.inner.epoch.value();
        let (epoch, epoch_block, epoch_block_count) = extract_epoch(epoch_field);

        let (average_block_time, estimated_epoch_time) =
            get_average_block_time_and_estimated_epoch_time(&tip_header, client)?;
        let overview_data = if self.enable_fetch_overview_data {
            let overview_data: Overview = client
                .post("get_overview", ())
                .with_context(|| anyhow!("Unable to get overview info"))?;
            Some(GetOverviewOfOverviewDashboardData {
                tx_pending: overview_data.pool.pending.value(),
                tx_proposed: overview_data.pool.proposed.value(),
                tx_committing: overview_data.pool.committing.value(),
                tx_rejected: overview_data.pool.total_recent_reject_num.value(),
                total_pool_size_in_bytes: overview_data.pool.total_tx_size.value(),
            })
        } else {
            None
        };
        *self = OverviewDashboardData {
            average_latency: -1,
            inbound_peers,
            outbound_peers,
            average_fee_rate: fee_rate_statistics.map(|x| x.mean.value()),
            epoch,
            epoch_block,
            epoch_block_count,
            average_block_time,
            estimated_epoch_time,
            enable_fetch_overview_data: self.enable_fetch_overview_data,
            overview_data,
        };
        log::info!("Updated: OverviewDashboardData");
        Ok(Box::new(self.clone()))
    }

    fn set_enable_overview_data(&mut self, flag: bool) {
        self.enable_fetch_overview_data = flag;
    }
}

pub fn basic_info_dashboard(_event_sender: mpsc::Sender<TUIEvent>) -> impl IntoBoxedView + use<> {
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
                                    .child(TextView::empty().with_name(COMMITTING_TX)),
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

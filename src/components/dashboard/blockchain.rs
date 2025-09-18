use anyhow::{Context, anyhow};
use ckb_sdk::CkbRpcClient;
use cursive::{
    view::{IntoBoxedView, Nameable, Resizable},
    views::{LinearLayout, NamedView, Panel, TextView},
};
use queue::Queue;

use crate::{
    components::{
        DashboardData, DashboardState, UpdateToView,
        dashboard::blockchain::names::{
            ALGORITHM, AVERAGE_BLOCK_TIME, BLOCK_HEIGHT, DIFFICULTY, EPOCH, ESTIMATED_EPOCH_TIME,
            HASH_RATE, LIVE_CELLS, LIVE_CELLS_HISTORY, OCCUPIED_CAPACITY,
            OCCUPIED_CAPACITY_HISTORY,
        },
        extract_epoch,
    },
    utils::bar_chart::SimpleBarChart,
};

const TEST_DATA: [f64; 10] = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];

mod names {
    pub const EPOCH: &str = "blockchain_dashboard_epoch";
    pub const ESTIMATED_EPOCH_TIME: &str = "blockchain_dashboard_estimated_epoch_time";
    pub const BLOCK_HEIGHT: &str = "blockchain_dashboard_block_height";
    pub const AVERAGE_BLOCK_TIME: &str = "blockchain_dashboard_estimated_epoch_time";
    pub const ALGORITHM: &str = "blockchain_dashboard_algorithm";
    pub const DIFFICULTY: &str = "blockchain_dashboard_difficulty";
    pub const HASH_RATE: &str = "blockchain_dashboard_hash_rate";

    pub const LIVE_CELLS: &str = "blockchain_dashboard_live_cells";
    pub const LIVE_CELLS_HISTORY: &str = "blockchain_dashboard_live_cells_history";
    pub const OCCUPIED_CAPACITY: &str = "blockchain_dashboard_occupied_capacity";
    pub const OCCUPIED_CAPACITY_HISTORY: &str = "blockchain_dashboard_occupied_capacity_history";
}

#[derive(Clone)]
pub struct BlockchainDashboardState {
    live_cells_history: Queue<f64>,
    max_live_cells: u64,
    live_cells: u64,

    occupied_capacity_history: Queue<f64>,
    max_occupied_capacity: u64,
    occupied_capacity: u64,

    client: CkbRpcClient,
}

impl UpdateToView for BlockchainDashboardState {
    fn update_to_view(&self, siv: &mut cursive::Cursive) {
        siv.call_on_name(LIVE_CELLS, |view: &mut TextView| {
            view.set_content(format!("{}", self.live_cells));
        });
        siv.call_on_name(LIVE_CELLS_HISTORY, |view: &mut SimpleBarChart| {
            view.set_max_value(self.max_live_cells as f64);
            view.set_data(self.live_cells_history.vec()).unwrap();
        });
        siv.call_on_name(OCCUPIED_CAPACITY, |view: &mut TextView| {
            view.set_content(format!("{} CKB", self.occupied_capacity));
        });
        siv.call_on_name(OCCUPIED_CAPACITY_HISTORY, |view: &mut SimpleBarChart| {
            view.set_max_value(self.max_occupied_capacity as f64);
            view.set_data(self.occupied_capacity_history.vec()).unwrap();
        });
    }
}

impl DashboardState for BlockchainDashboardState {
    fn update_state(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

impl BlockchainDashboardState {
    pub fn new(client: CkbRpcClient) -> Self {
        Self {
            client,
            live_cells: 1,
            live_cells_history: Queue::default(),
            max_live_cells: 1,
            max_occupied_capacity: 1,
            occupied_capacity: 1,
            occupied_capacity_history: Default::default(),
        }
    }
}

pub struct BlockchainDashboardData {
    epoch: u64,
    epoch_block: u64,
    epoch_block_count: u64,

    estimated_epoch_time: f64,
    average_block_time: f64,
    block_height: u64,
    algorithm: String,
    difficulty: f64,
    hash_rate: f64,
}

impl DashboardData for BlockchainDashboardData {
    fn fetch_data_through_client(client: &ckb_sdk::CkbRpcClient) -> anyhow::Result<Self> {
        let tip_header = client
            .get_tip_header()
            .with_context(|| anyhow!("Unable to get tip header"))?;
        let (epoch, epoch_block, epoch_block_count) = extract_epoch(tip_header.inner.epoch.value());
        Ok(Self {
            epoch,
            epoch_block,
            epoch_block_count,
            estimated_epoch_time: -1.0,
            average_block_time: -1.0,
            block_height: tip_header.inner.number.value(),
            algorithm: "Unknown".to_string(),
            difficulty: -1.0,
            hash_rate: -1.0,
        })
    }
}

impl UpdateToView for BlockchainDashboardData {
    fn update_to_view(&self, siv: &mut cursive::Cursive) {
        siv.call_on_name(EPOCH, |view: &mut TextView| {
            view.set_content(format!(
                "{} ({}/{})",
                self.epoch, self.epoch_block, self.epoch_block_count
            ));
        });
        siv.call_on_name(ESTIMATED_EPOCH_TIME, |view: &mut TextView| {
            view.set_content(format!("{} min", self.estimated_epoch_time / 60.0));
        });
        siv.call_on_name(BLOCK_HEIGHT, |view: &mut TextView| {
            view.set_content(format!("{}", self.block_height));
        });
        siv.call_on_name(AVERAGE_BLOCK_TIME, |view: &mut TextView| {
            view.set_content(format!("{} s", self.average_block_time));
        });
        siv.call_on_name(ALGORITHM, |view: &mut TextView| {
            view.set_content(format!("{}", self.algorithm));
        });
        siv.call_on_name(DIFFICULTY, |view: &mut TextView| {
            view.set_content(format!("{:.2} EH", self.difficulty));
        });

        siv.call_on_name(HASH_RATE, |view: &mut TextView| {
            view.set_content(format!("{:.2} PH/s", self.hash_rate));
        });
    }
}

pub fn blockchain_dashboard() -> impl IntoBoxedView + use<> {
    LinearLayout::vertical()
        .child(
            LinearLayout::horizontal()
                .child(
                    Panel::new(
                        LinearLayout::vertical()
                            .child(TextView::new("[Blockchain]"))
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
                                    .child(TextView::new("• Block Height:").min_width(20))
                                    .child(TextView::empty().with_name(BLOCK_HEIGHT)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• Avg. Block Time:").min_width(20))
                                    .child(TextView::empty().with_name(AVERAGE_BLOCK_TIME)),
                            ),
                    )
                    .min_width(50),
                )
                .child(
                    Panel::new(
                        LinearLayout::vertical()
                            .child(TextView::new("[Consensus]"))
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• Algorithm:").min_width(20))
                                    .child(TextView::empty().with_name(ALGORITHM)),
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
                    .min_width(50),
                ),
        )
        .child(Panel::new(
            LinearLayout::vertical()
                .child(TextView::new("[Cells]"))
                .child(
                    LinearLayout::horizontal()
                        .child(TextView::new("• Live Cells:").min_width(22))
                        .child(
                            TextView::new("Loading...")
                                .with_name(LIVE_CELLS)
                                .min_width(20),
                        )
                        .child(NamedView::new(
                            LIVE_CELLS_HISTORY,
                            SimpleBarChart::new(&TEST_DATA).unwrap(),
                        )),
                )
                .child(
                    LinearLayout::horizontal()
                        .child(TextView::new("• Occupied Capacity:").min_width(22))
                        .child(
                            TextView::new("Loading...")
                                .with_name(OCCUPIED_CAPACITY)
                                .min_width(20),
                        )
                        .child(NamedView::new(
                            OCCUPIED_CAPACITY_HISTORY,
                            SimpleBarChart::new(&TEST_DATA).unwrap(),
                        )),
                ),
        ))
}

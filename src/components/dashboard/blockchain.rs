use std::sync::mpsc;

use anyhow::{Context, anyhow};
use ckb_sdk::CkbRpcClient;
use cursive::{
    view::{IntoBoxedView, Nameable, Resizable, Scrollable},
    views::{LinearLayout, NamedView, Panel, TextView},
};
use cursive_table_view::{TableView, TableViewItem};
use queue::Queue;

use crate::{
    components::{
        dashboard::{blockchain::names::{
            ALGORITHM, AVERAGE_BLOCK_TIME, BLOCK_HEIGHT, DIFFICULTY, EPOCH, ESTIMATED_EPOCH_TIME,
            HASH_RATE, LIVE_CELLS, LIVE_CELLS_HISTORY, OCCUPIED_CAPACITY,
            OCCUPIED_CAPACITY_HISTORY, SCRIPT_TABLE,
        }, TUIEvent}, extract_epoch, get_average_block_time_and_estimated_epoch_time, DashboardData, DashboardState, UpdateToView
    }, declare_names, update_text, utils::bar_chart::SimpleBarChart, CURRENT_TAB
};

const TEST_DATA: [f64; 10] = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];

declare_names!(
    names,
    "blockchain_dashboard_",
    EPOCH,
    ESTIMATED_EPOCH_TIME,
    BLOCK_HEIGHT,
    AVERAGE_BLOCK_TIME,
    ALGORITHM,
    DIFFICULTY,
    HASH_RATE,
    LIVE_CELLS,
    LIVE_CELLS_HISTORY,
    OCCUPIED_CAPACITY,
    OCCUPIED_CAPACITY_HISTORY,
    SCRIPT_TABLE
);

#[derive(Clone)]
pub struct BlockchainDashboardState {
    live_cells_history: Queue<f64>,
    max_live_cells: u64,
    live_cells: u64,
    occupied_capacity_history: Queue<f64>,
    max_occupied_capacity: u64,
    occupied_capacity: u64,
}

impl UpdateToView for BlockchainDashboardState {
    fn update_to_view(&self, siv: &mut cursive::Cursive) {
        update_text!(siv, LIVE_CELLS, format!("{}", self.live_cells));
        siv.call_on_name(LIVE_CELLS_HISTORY, |view: &mut SimpleBarChart| {
            view.set_max_value(self.max_live_cells as f64);
            view.set_data(self.live_cells_history.vec()).unwrap();
        });
        update_text!(
            siv,
            OCCUPIED_CAPACITY,
            format!("{} CKB", self.occupied_capacity)
        );
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
    pub fn new(_client: CkbRpcClient) -> Self {
        Self {
            // client,
            live_cells: 1,
            live_cells_history: Queue::default(),
            max_live_cells: 1,
            max_occupied_capacity: 1,
            occupied_capacity: 1,
            occupied_capacity_history: Default::default(),
        }
    }
}
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
enum ScriptType {
    Lock,
    Type,
}

#[derive(Clone)]
struct ScriptItem {
    name: String,
    script_type: ScriptType,
    integrity: Result<(), String>,
    code_hash: String,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum ScriptColumn {
    Name,
    ScriptType,
    Integrity,
    CodeHash,
}

impl TableViewItem<ScriptColumn> for ScriptItem {
    fn to_column(&self, column: ScriptColumn) -> String {
        match column {
            ScriptColumn::Name => self.name.clone(),
            ScriptColumn::ScriptType => match self.script_type {
                ScriptType::Lock => String::from("Lock"),
                ScriptType::Type => String::from("Type"),
            },
            ScriptColumn::Integrity => match &self.integrity {
                Ok(_) => String::from("✓ OK"),
                Err(e) => e.clone(),
            },
            ScriptColumn::CodeHash => self.code_hash.clone(),
        }
    }

    fn cmp(&self, other: &Self, column: ScriptColumn) -> std::cmp::Ordering
    where
        Self: Sized,
    {
        match column {
            ScriptColumn::Name => self.name.cmp(&other.name),
            ScriptColumn::ScriptType => self.script_type.cmp(&other.script_type),
            ScriptColumn::Integrity => self.integrity.cmp(&other.integrity),
            ScriptColumn::CodeHash => self.code_hash.cmp(&other.code_hash),
        }
    }
}
#[derive(Clone, Default)]
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

    scripts: Vec<ScriptItem>,
}

impl DashboardData for BlockchainDashboardData {
    fn should_update(&self) -> bool {
        CURRENT_TAB.load(std::sync::atomic::Ordering::SeqCst) == 1
    }
    fn fetch_data_through_client(
        &mut self,
        client: &ckb_sdk::CkbRpcClient,
    ) -> anyhow::Result<Box<dyn DashboardData + Send + Sync>> {
        let tip_header = client
            .get_tip_header()
            .with_context(|| anyhow!("Unable to get tip header"))?;
        let (epoch, epoch_block, epoch_block_count) = extract_epoch(tip_header.inner.epoch.value());
        let (average_block_time, estimated_epoch_time) =
            get_average_block_time_and_estimated_epoch_time(&tip_header, client)?;
        let scripts = {
            let mut scripts = vec![];
            for i in 0..20 {
                scripts.push(ScriptItem {
                    name: format!("Script {}", i),
                    script_type: if i % 2 == 0 {
                        ScriptType::Lock
                    } else {
                        ScriptType::Type
                    },
                    integrity: Ok(()),
                    code_hash: format!("Code Hash {}", i),
                });
            }
            scripts
        };
        *self = Self {
            epoch,
            epoch_block,
            epoch_block_count,
            estimated_epoch_time,
            average_block_time,
            block_height: tip_header.inner.number.value(),
            algorithm: "Unknown".to_string(),
            difficulty: -1.0,
            hash_rate: -1.0,
            scripts,
        };
        Ok(Box::new(self.clone()))
    }
}

impl UpdateToView for BlockchainDashboardData {
    fn update_to_view(&self, siv: &mut cursive::Cursive) {
        update_text!(
            siv,
            EPOCH,
            format!(
                "{} ({}/{})",
                self.epoch, self.epoch_block, self.epoch_block_count
            )
        );
        update_text!(
            siv,
            ESTIMATED_EPOCH_TIME,
            format!("{:.2} min", self.estimated_epoch_time / 60.0)
        );
        update_text!(siv, BLOCK_HEIGHT, format!("{}", self.block_height));
        update_text!(
            siv,
            AVERAGE_BLOCK_TIME,
            format!("{:.2} s", self.average_block_time)
        );
        update_text!(siv, ALGORITHM, format!("{}", self.algorithm));
        update_text!(siv, DIFFICULTY, format!("{:.2} EH", self.difficulty));
        update_text!(siv, HASH_RATE, format!("{:.2} PH/s", self.hash_rate));
        siv.call_on_name(
            SCRIPT_TABLE,
            |view: &mut TableView<ScriptItem, ScriptColumn>| {
                let index = view.row();
                view.clear();
                for i in 0..self.scripts.len() {
                    view.insert_item(self.scripts[i].clone());
                }
                if let Some(index) = index {
                    view.set_selected_row(index);
                }
            },
        );
    }
}

pub fn blockchain_dashboard(_event_sender: mpsc::Sender<TUIEvent>) -> impl IntoBoxedView + use<> {
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
                    .min_width(50)
                    .scrollable(),
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
                    .min_width(50)
                    .scrollable(),
                ),
        )
        .child(
            Panel::new(
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
            )
            .scrollable(),
        )
        .child(
            Panel::new(
                LinearLayout::vertical()
                    .child(TextView::new("[Script Integrity]"))
                    .child(TextView::new(" "))
                    .child(
                        TableView::<ScriptItem, ScriptColumn>::new()
                            .column(ScriptColumn::Name, "System Script Name", |c| c)
                            .column(ScriptColumn::ScriptType, "Lock/Type Script", |c| c)
                            .column(ScriptColumn::Integrity, "Integrity Check", |c| c)
                            .column(ScriptColumn::CodeHash, "Code Hash", |c| c)
                            .with_name(SCRIPT_TABLE)
                            .min_size((50, 20)),
                    ),
            )
            .scrollable(),
        )
}

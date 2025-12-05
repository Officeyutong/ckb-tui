use std::sync::{Arc, RwLock, mpsc};

use anyhow::{Context, anyhow};
use chrono::{DateTime, Local, TimeZone, Utc};
use ckb_fixed_hash_core::H256;
use ckb_jsonrpc_types::{BlockView, Consensus};
use ckb_jsonrpc_types_new::Overview;
use ckb_sdk::CkbRpcClient;
use cursive::{
    view::{IntoBoxedView, Nameable, Resizable, Scrollable},
    views::{Button, Dialog, LinearLayout, ListView, NamedView, Panel, TextView},
};
use cursive_table_view::{TableView, TableViewItem};
use queue::Queue;
use thousands::Separable;
use tokio_stream::StreamExt;

use crate::{
    CURRENT_TAB,
    components::{
        DashboardData, DashboardState, UpdateToView,
        dashboard::{
            TUIEvent,
            blockchain::names::{
                ALGORITHM, AVERAGE_BLOCK_TIME, BLOCK_HEIGHT, BLOCKS_SUBSCRIPTION_WARNING,
                BLOCKS_TABLE, DIFFICULTY, EPOCH, ESTIMATED_EPOCH_TIME, HASH_RATE, LIVE_CELLS,
                LIVE_CELLS_HISTORY, OCCUPIED_CAPACITY, OCCUPIED_CAPACITY_HISTORY, SCRIPT_TABLE,
            },
        },
        extract_epoch, get_average_block_time_and_estimated_epoch_time,
    },
    declare_names, update_text,
    utils::{
        bar_chart::SimpleBarChart, create_subscription_client, difficulty_to_string,
        hash_rate_to_string, shorten_hex,
    },
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
    SCRIPT_TABLE,
    BLOCKS_SUBSCRIPTION_WARNING,
    BLOCKS_TABLE
);

#[derive(Clone, Default)]
pub struct GetOverviewOfBlockchainDasboardState {
    live_cells_history: Queue<f64>,
    max_live_cells: u64,
    min_live_cells: u64,
    live_cells: u64,
    occupied_capacity_history: Queue<f64>,
    max_occupied_capacity: u64,
    min_occupied_capacity: u64,
    occupied_capacity: u64,
}
#[derive(Clone)]
struct BlockListItem {
    time: DateTime<Local>,
    block_number: u64,
    block_hash: H256,
}
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum BlockListColumn {
    Time,
    BlockNumber,
    BlockHash,
}

impl TableViewItem<BlockListColumn> for BlockListItem {
    fn to_column(&self, column: BlockListColumn) -> String {
        match column {
            BlockListColumn::Time => format!(
                "{}s ago",
                chrono::Local::now().timestamp() - self.time.timestamp()
            ),
            BlockListColumn::BlockNumber => self.block_number.to_string(),
            BlockListColumn::BlockHash => shorten_hex(self.block_hash.to_string(), 5, 5),
        }
    }

    fn cmp(&self, other: &Self, column: BlockListColumn) -> std::cmp::Ordering
    where
        Self: Sized,
    {
        match column {
            BlockListColumn::Time => self.time.cmp(&other.time).reverse(),
            BlockListColumn::BlockNumber => self.block_number.cmp(&other.block_number),
            BlockListColumn::BlockHash => self.block_hash.cmp(&other.block_hash),
        }
    }
}

#[derive(Clone)]
pub struct BlockchainDashboardState {
    client: CkbRpcClient,
    consensus: Option<Consensus>,
    overview_data: Option<GetOverviewOfBlockchainDasboardState>,
    subscription: BlockChainDashboardSubscriptionState,
}
#[derive(Clone)]
pub struct BlockChainDashboardWithTcpConnState {
    blocks: Arc<RwLock<Queue<BlockListItem>>>,
    stop_tx: tokio::sync::mpsc::Sender<()>,
}
fn update_blocks(state: &BlockChainDashboardWithTcpConnState, block_view: BlockView) {
    let mut guard = state.blocks.write().unwrap();
    guard
        .queue(BlockListItem {
            time: Utc
                .timestamp_millis_opt(block_view.header.inner.timestamp.value() as i64)
                .unwrap()
                .into(),
            block_number: block_view.header.inner.number.value(),
            block_hash: block_view.header.hash,
        })
        .unwrap();
    if guard.len() > 10 {
        guard.dequeue();
    }
}

#[derive(Clone)]
pub enum BlockChainDashboardSubscriptionState {
    WithTcpConn(BlockChainDashboardWithTcpConnState),
    WithoutTcpConn,
}

impl UpdateToView for BlockchainDashboardState {
    fn update_to_view(&self, siv: &mut cursive::Cursive) {
        if let Some(data) = &self.overview_data {
            update_text!(
                siv,
                LIVE_CELLS,
                format!("{}", data.live_cells.separate_with_commas())
            );
            siv.call_on_name(LIVE_CELLS_HISTORY, |view: &mut SimpleBarChart| {
                view.set_max_value(data.max_live_cells as f64);
                view.set_diff_value(Some(data.min_live_cells as f64 * 0.9));
                view.set_data(data.live_cells_history.vec()).unwrap();
            });
            update_text!(
                siv,
                OCCUPIED_CAPACITY,
                format!("{} CKB ", data.occupied_capacity.separate_with_commas())
            );
            siv.call_on_name(OCCUPIED_CAPACITY_HISTORY, |view: &mut SimpleBarChart| {
                view.set_max_value(data.max_occupied_capacity as f64);
                view.set_diff_value(Some(data.min_occupied_capacity as f64 * 0.9));
                view.set_data(data.occupied_capacity_history.vec()).unwrap();
            });
        } else {
            update_text!(siv, LIVE_CELLS, format!("N/A"));
            siv.call_on_name(LIVE_CELLS_HISTORY, |view: &mut SimpleBarChart| {
                view.set_data(&[]).unwrap();
            });
            update_text!(siv, OCCUPIED_CAPACITY, format!("N/A"));
            siv.call_on_name(OCCUPIED_CAPACITY_HISTORY, |view: &mut SimpleBarChart| {
                view.set_data(&[]).unwrap();
            });
        }
        if let BlockChainDashboardSubscriptionState::WithTcpConn(conn_data) = &self.subscription {
            siv.call_on_name(
                BLOCKS_TABLE,
                |view: &mut TableView<BlockListItem, BlockListColumn>| {
                    view.set_items(conn_data.blocks.read().unwrap().vec().clone());
                },
            );
        } else {
            siv.call_on_name(BLOCKS_SUBSCRIPTION_WARNING, |view:&mut TextView|{
                view.set_content( "Subscribe TCP address is not set, latest transactions and rejected transactions won't be updated");
            });
        }
    }
}

impl DashboardState for BlockchainDashboardState {
    fn accept_event(&mut self, event: &TUIEvent) {
        if let TUIEvent::OpenConsensusModal(sender) = event {
            if let Some(consensus) = self.consensus.clone() {
                sender
                    .send(Box::new(move |siv| {
                        siv.add_layer(consensus_modal(&consensus));
                    }))
                    .unwrap();
            }
        }
    }
    fn update_state(&mut self) -> anyhow::Result<()> {
        if let Some(data) = &mut self.overview_data {
            let overview = self
                .client
                .post::<(), Overview>("get_overview", ())
                .with_context(|| anyhow!("Unable to get overview data"))?;
            let occupied_capacity = overview.cells.total_occupied_capacities.value();
            data.max_occupied_capacity = data.max_occupied_capacity.max(occupied_capacity);
            data.min_occupied_capacity = data.min_occupied_capacity.min(occupied_capacity);
            data.occupied_capacity = occupied_capacity;
            data.occupied_capacity_history
                .queue(occupied_capacity as f64)
                .unwrap();
            if data.occupied_capacity_history.len() > 20 {
                data.occupied_capacity_history.dequeue();
            }

            let live_cells = overview.cells.estimate_live_cells_num.value();
            data.max_live_cells = data.max_live_cells.max(live_cells);
            data.min_live_cells = data.min_live_cells.min(live_cells);
            data.live_cells = live_cells;
            data.live_cells_history.queue(live_cells as f64).unwrap();
            if data.live_cells_history.len() > 20 {
                data.live_cells_history.dequeue();
            }
        }

        self.consensus = Some(
            self.client
                .get_consensus()
                .with_context(|| anyhow!("Unable to get consensus"))?,
        );

        Ok(())
    }
}

impl BlockchainDashboardState {
    #[allow(unused)]
    pub fn stop(&self) {
        match &self.subscription {
            BlockChainDashboardSubscriptionState::WithTcpConn(
                block_chain_dashboard_with_tcp_conn_state,
            ) => block_chain_dashboard_with_tcp_conn_state
                .stop_tx
                .blocking_send(())
                .unwrap(),
            BlockChainDashboardSubscriptionState::WithoutTcpConn => {}
        };
    }
    pub fn new(
        client: CkbRpcClient,
        fetch_overview_data: bool,
        subscription_url: Option<String>,
    ) -> Self {
        let subscription = if let Some(url) = subscription_url {
            let (stop_tx, mut stop_rx) = tokio::sync::mpsc::channel(1);
            let result = BlockChainDashboardSubscriptionState::WithTcpConn(
                BlockChainDashboardWithTcpConnState {
                    blocks: Arc::new(RwLock::new(Queue::new())),
                    stop_tx,
                },
            );
            let self_cloned = result.clone();
            std::thread::spawn(move || {
                log::info!("Subscription thread of blockchain started");
                let result = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(o) => o,
                    Err(e) => {
                        log::error!("{:?}", e);
                        panic!("Unable to start tokio runtime");
                    }
                }
                .block_on(async move {
                    let mut block_sub = create_subscription_client(&url)
                        .await
                        .with_context(|| anyhow!("Unable to connect to:{}", url))?
                        .subscribe::<BlockView>("new_tip_block")
                        .await
                        .with_context(|| anyhow!("Unable to subscribe new blocks"))?;
                    loop {
                        tokio::select! {
                            _ = stop_rx.recv() => {
                                log::trace!("Exiting tx subscribing thread");
                                break;
                            }
                            Some(Ok(r)) = block_sub.next() => {
                                log::trace!("Received block sub: {:?}", r);
                                update_blocks(match self_cloned {
                                    BlockChainDashboardSubscriptionState::WithTcpConn(ref  block_chain_dashboard_with_tcp_conn_state) => block_chain_dashboard_with_tcp_conn_state,
                                    BlockChainDashboardSubscriptionState::WithoutTcpConn => unreachable!(),
                                }, r.1);
                            }
                        }
                    }
                    anyhow::Ok(())
                });
                log::info!(
                    "Tokio runtime of blockchain subscription exited: {:?}",
                    result
                );
            });
            result
        } else {
            BlockChainDashboardSubscriptionState::WithoutTcpConn
        };

        Self {
            client,
            consensus: None,
            overview_data: if fetch_overview_data {
                Some(Default::default())
            } else {
                None
            },
            subscription,
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
            ScriptColumn::CodeHash => shorten_hex(&self.code_hash, 6, 5),
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

#[derive(Clone)]
pub struct GetOverviewOfBlockchainDashboardData {
    difficulty: f64,
    hash_rate: f64,
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

    overview_data: Option<GetOverviewOfBlockchainDashboardData>,

    scripts: Vec<ScriptItem>,

    enable_overview_data: bool,
}

impl DashboardData for BlockchainDashboardData {
    fn set_enable_overview_data(&mut self, flag: bool) {
        self.enable_overview_data = flag;
    }
    fn should_update(&self) -> bool {
        CURRENT_TAB.load(std::sync::atomic::Ordering::SeqCst) == 1
    }
    fn fetch_data_through_client(
        &mut self,
        client: &ckb_sdk::CkbRpcClient,
    ) -> anyhow::Result<Box<dyn DashboardData + Send + Sync>> {
        log::debug!("Updating: BlockchainDashboardData");
        let tip_header = client
            .get_tip_header()
            .with_context(|| anyhow!("Unable to get tip header"))?;
        let (epoch, epoch_block, epoch_block_count) = extract_epoch(tip_header.inner.epoch.value());
        let (average_block_time, estimated_epoch_time) =
            get_average_block_time_and_estimated_epoch_time(&tip_header, client)?;
        let overview_data = if self.enable_overview_data {
            let data = client.post::<(), Overview>("get_overview", ())?;
            Some(GetOverviewOfBlockchainDashboardData {
                difficulty: data.mining.difficulty.to_string().parse::<f64>().unwrap(),
                hash_rate: data.mining.hash_rate.to_string().parse::<f64>().unwrap(),
            })
        } else {
            None
        };
        let consensus = client
            .get_consensus()
            .with_context(|| anyhow!("Unable to get consensus"))?;

        let scripts = {
            let mut scripts = vec![];
            if let Some(hash) = consensus.secp256k1_blake160_sighash_all_type_hash {
                scripts.push(ScriptItem {
                    name: String::from("secp256k1_blake160_sighash_all"),
                    script_type: ScriptType::Lock,
                    integrity: Ok(()),
                    code_hash: hash.to_string(),
                });
            }
            if let Some(hash) = consensus.secp256k1_blake160_multisig_all_type_hash {
                scripts.push(ScriptItem {
                    name: String::from("secp256k1_blake160_multisig_all"),
                    script_type: ScriptType::Lock,
                    integrity: Ok(()),
                    code_hash: hash.to_string(),
                });
            }
            scripts.push(ScriptItem {
                name: String::from("dao"),
                script_type: ScriptType::Lock,
                integrity: Ok(()),
                code_hash: consensus.dao_type_hash.to_string(),
            });
            scripts.push(ScriptItem {
                name: String::from("type_id"),
                script_type: ScriptType::Type,
                integrity: Ok(()),
                code_hash: consensus.type_id_code_hash.to_string(),
            });
            scripts
        };
        *self = Self {
            epoch,
            epoch_block,
            epoch_block_count,
            estimated_epoch_time,
            average_block_time,
            block_height: tip_header.inner.number.value(),
            algorithm: "Eaglesong".to_string(),
            enable_overview_data: self.enable_overview_data,
            overview_data,
            scripts,
        };
        log::debug!("Updated: MempoolDashboardData");
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
        if let Some(data) = &self.overview_data {
            update_text!(siv, DIFFICULTY, difficulty_to_string(data.difficulty));
            update_text!(siv, HASH_RATE, hash_rate_to_string(data.hash_rate));
        } else {
            update_text!(siv, DIFFICULTY, format!("N/A"));
            update_text!(siv, HASH_RATE, format!("N/A"));
        }
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

pub fn blockchain_dashboard(event_sender: mpsc::Sender<TUIEvent>) -> impl IntoBoxedView + use<> {
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
                            )
                            .child(Button::new("Consensus", move |siv| {
                                event_sender
                                    .send(TUIEvent::OpenConsensusModal(siv.cb_sink().clone()))
                                    .unwrap();
                            })),
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
                    .child(TextView::new(" "))
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
                            .on_submit(|siv, _row, index| {
                                let line = siv
                                    .call_on_name(
                                        SCRIPT_TABLE,
                                        |view: &mut TableView<ScriptItem, ScriptColumn>| {
                                            view.borrow_item(index).unwrap().clone()
                                        },
                                    )
                                    .unwrap();
                                siv.add_layer(script_detail_modal(&line));
                            })
                            .with_name(SCRIPT_TABLE)
                            .min_size((100, 7)),
                    ),
            )
            .scrollable(),
        )
        .child(Panel::new(
            LinearLayout::vertical()
                .child(TextView::new("[Latest Blocks]"))
                .child(TextView::new(" ").with_name(BLOCKS_SUBSCRIPTION_WARNING))
                .child(
                    TableView::<BlockListItem, BlockListColumn>::new()
                        .column(BlockListColumn::Time, "Time", |c| c)
                        .column(BlockListColumn::BlockNumber, "Block Number", |c| c)
                        .column(BlockListColumn::BlockHash, "Block Hash", |c| c)
                        .on_submit(|siv, _row, index| {
                            let line = siv
                                .call_on_name(
                                    BLOCKS_TABLE,
                                    |view: &mut TableView<BlockListItem, BlockListColumn>| {
                                        view.borrow_item(index).unwrap().clone()
                                    },
                                )
                                .unwrap();
                            siv.add_layer(block_modal(&line));
                        })
                        .with_name(BLOCKS_TABLE)
                        .min_size((100, 8)),
                )
                .scrollable(),
        ))
}

fn script_detail_modal(data: &ScriptItem) -> impl IntoBoxedView + use<> {
    Dialog::around(
        LinearLayout::vertical()
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("• System Script Name:").min_width(25))
                    .child(TextView::new(&data.name)),
            )
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("• Code Hash:").min_width(25))
                    .child(TextView::new(&data.code_hash)),
            )
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("• Integrity Name:").min_width(25))
                    .child(TextView::new(&match &data.integrity {
                        Ok(()) => String::from("Ok"),
                        Err(e) => e.to_string(),
                    })),
            )
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("• Script Type:").min_width(25))
                    .child(TextView::new(match &data.script_type {
                        ScriptType::Lock => "Lock",
                        ScriptType::Type => "Type",
                    })),
            ),
    )
    .title("Details of Script")
    .button("Close", |siv| {
        siv.pop_layer();
    })
}

fn consensus_modal(data: &Consensus) -> impl IntoBoxedView + use<> {
    Dialog::around(
        LinearLayout::vertical()
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("• Initial primary epoch reward:").min_width(40))
                    .child(TextView::new(format!(
                        "{}",
                        data.initial_primary_epoch_reward.value()
                    ))),
            )
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("• Secondary epoch reward:").min_width(40))
                    .child(TextView::new(format!(
                        "{}",
                        data.secondary_epoch_reward.value()
                    ))),
            )
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("• Max block cycles:").min_width(40))
                    .child(TextView::new(format!("{}", data.max_block_cycles.value()))),
            )
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("• Cellbase maturity:").min_width(40))
                    .child(TextView::new(format!("{}", data.cellbase_maturity.value()))),
            )
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("• Primary epoch reward halving interval:").min_width(40))
                    .child(TextView::new(format!(
                        "{}",
                        data.primary_epoch_reward_halving_interval.value()
                    ))),
            )
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("• Epoch duration target:").min_width(40))
                    .child(TextView::new(format!(
                        "{}",
                        data.epoch_duration_target.value()
                    ))),
            )
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("• Permanent difficulty in dummy:").min_width(40))
                    .child(TextView::new(format!(
                        "{}",
                        data.permanent_difficulty_in_dummy
                    ))),
            ),
    )
    .title("Consensus")
    .button("Close", |siv| {
        siv.pop_layer();
    })
}

fn block_modal(data: &BlockListItem) -> impl IntoBoxedView {
    Dialog::around(
        ListView::new()
            .child("Block Hash", TextView::new(data.block_hash.to_string()))
            .child("Block Number", TextView::new(data.block_number.to_string()))
            .child("Time", TextView::new(data.time.to_rfc2822())),
    )
    .title("Details of block")
    .button("Close", |siv| {
        siv.pop_layer();
    })
}

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::AtomicUsize;

use anyhow::Context;
use anyhow::anyhow;
use chrono::Local;
use chrono::TimeZone;
use chrono::Utc;
use ckb_jsonrpc_types::PoolTransactionEntry;
use ckb_jsonrpc_types::PoolTransactionReject;
use ckb_jsonrpc_types_new::Overview;
use ckb_sdk::CkbRpcClient;
use cursive::view::Scrollable;
use cursive::{
    view::{IntoBoxedView, Nameable, Resizable},
    views::{LinearLayout, Panel, TextView},
};
use cursive_table_view::{TableView, TableViewItem};
use queue::Queue;
use std::sync::mpsc;
use tokio::net::TcpStream;
use tokio_stream::StreamExt;

use crate::components::DashboardState;
use crate::components::dashboard::TUIEvent;
use crate::components::dashboard::mempool::names::SUBSCRIBE_WARNING;
use crate::components::get_average_block_time_and_estimated_epoch_time;
use crate::components::map_pool_transaction_to_reason;
use crate::utils::shorten_hex;
use crate::{
    CURRENT_TAB,
    components::{
        DashboardData, UpdateToView,
        dashboard::mempool::names::{
            AVG_BLOCK_TIME, AVG_FEE_RATE, COMMITTING, LATEST_INCOMING_TX_TABLE, PENDING, PROPOSED,
            REJECTION_RATE, REJECTION_TABLE, TOTAL_POOL_SIZE, TOTAL_REJECTION, TX_IN, TX_OUT,
        },
    },
    declare_names, update_text,
};

declare_names!(
    names,
    "mempool_dashboard_",
    TOTAL_POOL_SIZE,
    PENDING,
    PROPOSED,
    COMMITTING,
    AVG_FEE_RATE,
    TX_IN,
    TX_OUT,
    AVG_BLOCK_TIME,
    TOTAL_REJECTION,
    REJECTION_RATE,
    REJECTION_TABLE,
    LATEST_INCOMING_TX_TABLE,
    SUBSCRIBE_WARNING
);

#[derive(Clone)]
pub struct MempoolDashboatdInnerState {
    total_rejection: Arc<AtomicUsize>,
    total_transaction: Arc<AtomicUsize>,
    rejection_details: Arc<RwLock<HashMap<String, usize>>>,
    latest_incoming_txs: Arc<RwLock<Queue<LatestIncomingTxItem>>>,
    stop_tx: tokio::sync::mpsc::Sender<()>,
}

#[derive(Clone)]
pub enum MempoolDashboardState {
    WithTcpConn(MempoolDashboatdInnerState),
    WithoutTcpConn,
}

async fn create_client(addr: &str) -> anyhow::Result<ckb_sdk::pubsub::Client<TcpStream>> {
    log::debug!("Connecting TCP: {}", addr);
    Ok(ckb_sdk::pubsub::Client::new(
        TcpStream::connect(addr).await?,
    ))
}

fn update_latest_tx(state: &MempoolDashboatdInnerState, tx: PoolTransactionEntry) {
    let mut guard = state.latest_incoming_txs.write().unwrap();
    guard
        .queue(LatestIncomingTxItem {
            tx_hash: shorten_hex(tx.transaction.hash.to_string(), 10, 11),
            time: Utc
                .timestamp_millis_opt(tx.timestamp.value() as i64)
                .unwrap()
                .into(),
            size_in_bytes: tx.size.value(),
            fee_rate: tx.fee.value() * 1000 / tx.size.value(),
        })
        .unwrap();
    if guard.len() > 20 {
        guard.dequeue();
    }
    state
        .total_transaction
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
}

fn update_rejected_tx(state: &MempoolDashboatdInnerState, rej_tx: PoolTransactionReject) {
    let mut guard = state.rejection_details.write().unwrap();
    let reason = map_pool_transaction_to_reason(&rej_tx);
    match guard.entry(reason.to_string()) {
        std::collections::hash_map::Entry::Occupied(mut occupied_entry) => {
            *occupied_entry.get_mut() += 1;
        }
        std::collections::hash_map::Entry::Vacant(vacant_entry) => {
            vacant_entry.insert(1);
        }
    };
    state.total_rejection.store(
        guard.iter().map(|x| *x.1).sum(),
        std::sync::atomic::Ordering::SeqCst,
    );
}

impl MempoolDashboardState {
    #[allow(unused)]
    pub fn stop(&self) {
        match self {
            MempoolDashboardState::WithTcpConn(mempool_dashboatd_inner_state) => {
                mempool_dashboatd_inner_state.stop_tx.blocking_send(()).ok();
            }
            MempoolDashboardState::WithoutTcpConn => {}
        };
    }
    pub fn new(subscribe_addr: Option<String>) -> Self {
        if let Some(subscribe_addr) = subscribe_addr {
            let (stop_tx, mut stop_rx) = tokio::sync::mpsc::channel(1);
            let result = Self::WithTcpConn(MempoolDashboatdInnerState {
                total_rejection: Arc::new(AtomicUsize::new(0)),
                total_transaction: Arc::new(AtomicUsize::new(0)),
                rejection_details: Arc::new(RwLock::new(HashMap::new())),
                latest_incoming_txs: Arc::new(RwLock::new(Queue::new())),
                stop_tx,
            });
            let self_cloned = result.clone();
            let tcp_addr = subscribe_addr.to_string();
            std::thread::spawn(move || {
                log::info!("Subscribing thread started");

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
                    let mut new_tx_sub = create_client(&tcp_addr)
                        .await
                        .with_context(|| anyhow!("Unable to connect to: {}", tcp_addr))?
                        .subscribe::<PoolTransactionEntry>("new_transaction")
                        .await
                        .with_context(|| anyhow!("Unable to subscribe new_transaction"))?;
                    let mut new_rejection_sub = create_client(&tcp_addr)
                        .await
                        .with_context(|| anyhow!("Unable to connect to: {}", tcp_addr))?
                        .subscribe::<(PoolTransactionEntry, PoolTransactionReject)>(
                            "rejected_transaction",
                        )
                        .await
                        .with_context(|| anyhow!("Unable to subscribe rejected_transaction"))?;
                    log::info!("Before subscribe select loop");
                    loop {
                        tokio::select! {
                            _ = stop_rx.recv() => {
                                log::debug!("Exiting tx subscribing thread");
                                break;
                            }
                            Some(Ok(r)) = new_tx_sub.next() => {
                                log::debug!("Received transaction sub: {:?}", r);
                                update_latest_tx(match self_cloned{
                                    MempoolDashboardState::WithTcpConn(ref mempool_dashboatd_inner_state) => mempool_dashboatd_inner_state,
                                    MempoolDashboardState::WithoutTcpConn => unreachable!(),
                                }, r.1);
                            }
                            Some(Ok(r)) = new_rejection_sub.next() => {
                                log::debug!("Received rejected tx sub: {:?}", r);
                                update_rejected_tx(match self_cloned{
                                    MempoolDashboardState::WithTcpConn(ref mempool_dashboatd_inner_state) => mempool_dashboatd_inner_state,
                                    MempoolDashboardState::WithoutTcpConn => unreachable!(),
                                }, r.1.1);
                            }
                        }
                    }
                    anyhow::Ok(())
                });
                log::info!("Tokio runtime exited: {:?}", result);
            });
            result
        } else {
            Self::WithoutTcpConn
        }
    }
}

impl DashboardState for MempoolDashboardState {
    fn update_state(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
impl UpdateToView for MempoolDashboardState {
    fn update_to_view(&self, siv: &mut cursive::Cursive) {
        match self {
            MempoolDashboardState::WithTcpConn(state) => {
                update_text!(
                    siv,
                    TOTAL_REJECTION,
                    format!(
                        "{}",
                        state
                            .total_rejection
                            .load(std::sync::atomic::Ordering::SeqCst)
                    )
                );
                siv.call_on_name(
                    REJECTION_TABLE,
                    |v: &mut TableView<RejectionItem, RejectionColumn>| {
                        v.clear();
                        let guard = state.rejection_details.read().unwrap();

                        let mut items = guard.iter().collect::<Vec<_>>();
                        items.sort_by(|(_, x), (_, y)| x.cmp(y).reverse());
                        for (reason, count) in items.into_iter() {
                            v.insert_item(RejectionItem {
                                reason: reason.to_string(),
                                count: *count,
                            });
                        }
                    },
                );
                siv.call_on_name(
                    LATEST_INCOMING_TX_TABLE,
                    |v: &mut TableView<LatestIncomingTxItem, LatestIncomingTxColumn>| {
                        let index = v.row();
                        v.clear();
                        for item in state.latest_incoming_txs.read().unwrap().vec().iter() {
                            v.insert_item(item.clone());
                        }
                        if let Some(index) = index {
                            v.set_selected_row(index);
                        }
                    },
                );
                update_text!(siv, SUBSCRIBE_WARNING, " ");
                let rejection_rate = state
                    .total_rejection
                    .load(std::sync::atomic::Ordering::SeqCst)
                    as f64
                    / state
                        .total_transaction
                        .load(std::sync::atomic::Ordering::SeqCst).max(1) /*Avoid NaN*/ as f64;
                update_text!(
                    siv,
                    REJECTION_RATE,
                    format!("{:.2} %", rejection_rate * 100.0)
                );
            }
            MempoolDashboardState::WithoutTcpConn => {
                update_text!(
                    siv,
                    SUBSCRIBE_WARNING,
                    "Subscribe TCP address is not set, latest transactions and rejected transactions won't be updated"
                );
            }
        }
    }
}

#[derive(Clone)]
pub struct GetOverviewOfMempoolDashboardData {
    total_pool_size_in_bytes: u64,
    pending_tx: u64,
    proposed_tx: u64,
    committing_tx: u64,
}

#[derive(Clone, Default)]
pub struct MempoolDashboardData {
    avg_fee_rate: Option<u64>,
    tx_in: usize,
    tx_out: usize,
    average_block_time: f64,
    overview_data: Option<GetOverviewOfMempoolDashboardData>,
    enable_fetch_overview: bool,
}

impl DashboardData for MempoolDashboardData {
    fn fetch_data_through_client(
        &mut self,
        client: &CkbRpcClient,
    ) -> anyhow::Result<Box<dyn DashboardData + Send + Sync>> {
        log::info!("Updating: MempoolDashboardData");
        let fee_rate_statistics = client
            .get_fee_rate_statistics(None)
            .with_context(|| anyhow!("Unable to get fee rate statistics"))?;
        let tip_header = client
            .get_tip_header()
            .with_context(|| anyhow!("Unable to get tip header"))?;
        let overview_data = if self.enable_fetch_overview {
            let overview: Overview = client
                .post("get_overview", ())
                .with_context(|| anyhow!("Unable to get overview info"))?;
            Some(GetOverviewOfMempoolDashboardData {
                total_pool_size_in_bytes: overview.pool.total_tx_size.value(),
                pending_tx: overview.pool.pending.value(),
                proposed_tx: overview.pool.proposed.value(),
                committing_tx: overview.pool.committing.value(),
            })
        } else {
            None
        };
        let (average_block_time, _) =
            get_average_block_time_and_estimated_epoch_time(&tip_header, client)?;
        *self = Self {
            overview_data,
            avg_fee_rate: fee_rate_statistics.map(|x| x.mean.value()),
            tx_in: 0,
            tx_out: 0,
            average_block_time,
            enable_fetch_overview: self.enable_fetch_overview,
        };
        log::info!("Updated: PeersDashboardData");
        Ok(Box::new(self.clone()))
    }
    fn should_update(&self) -> bool {
        CURRENT_TAB.load(std::sync::atomic::Ordering::SeqCst) == 2
    }

    fn set_enable_overview_data(&mut self, flag: bool) {
        self.enable_fetch_overview = flag;
    }
}

impl UpdateToView for MempoolDashboardData {
    fn update_to_view(&self, siv: &mut cursive::Cursive) {
        if let Some(data) = &self.overview_data {
            update_text!(
                siv,
                TOTAL_POOL_SIZE,
                format!(
                    "{} txs ({:.1} MB)",
                    data.pending_tx + data.committing_tx + data.proposed_tx,
                    data.total_pool_size_in_bytes as f64 / 1024.0 / 1024.0
                )
            );
            update_text!(siv, PENDING, format!("{}", data.pending_tx));
            update_text!(siv, PROPOSED, format!("{}", data.proposed_tx));
            update_text!(siv, COMMITTING, format!("{}", data.committing_tx));
        } else {
            update_text!(siv, TOTAL_POOL_SIZE, format!("N/A",));
            update_text!(siv, PENDING, format!("N/A"));
            update_text!(siv, PROPOSED, format!("N/A"));
            update_text!(siv, COMMITTING, format!("N/A"));
        };
        update_text!(
            siv,
            AVG_FEE_RATE,
            match self.avg_fee_rate {
                None => format!("N/A"),
                Some(v) => format!("{} shannons/KB", v),
            }
        );
        update_text!(siv, TX_IN, format!("{} tx/s", self.tx_in));
        update_text!(siv, TX_OUT, format!("{} tx/s", self.tx_out));
        update_text!(
            siv,
            AVG_BLOCK_TIME,
            format!("{:.1}s", self.average_block_time)
        );
    }
}
#[derive(Clone, Default)]
struct RejectionItem {
    reason: String,
    count: usize,
}
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum RejectionColumn {
    Reason,
    Count,
}

impl TableViewItem<RejectionColumn> for RejectionItem {
    fn to_column(&self, column: RejectionColumn) -> String {
        match column {
            RejectionColumn::Reason => self.reason.clone(),
            RejectionColumn::Count => format!("{}", self.count),
        }
    }

    fn cmp(&self, other: &Self, column: RejectionColumn) -> std::cmp::Ordering
    where
        Self: Sized,
    {
        match column {
            RejectionColumn::Reason => self.reason.cmp(&other.reason),
            RejectionColumn::Count => self.count.cmp(&other.count),
        }
    }
}
#[derive(Clone)]
struct LatestIncomingTxItem {
    tx_hash: String,
    time: chrono::DateTime<Local>,
    size_in_bytes: u64,
    fee_rate: u64,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum LatestIncomingTxColumn {
    TxHash,
    Time,
    SizeInBytes,
    FeeRate,
}

impl TableViewItem<LatestIncomingTxColumn> for LatestIncomingTxItem {
    fn to_column(&self, column: LatestIncomingTxColumn) -> String {
        match column {
            LatestIncomingTxColumn::TxHash => self.tx_hash.clone(),
            LatestIncomingTxColumn::Time => format!(
                "{}s ago",
                chrono::Local::now().timestamp() - self.time.timestamp()
            ),
            LatestIncomingTxColumn::SizeInBytes => format!("{}", self.size_in_bytes),
            LatestIncomingTxColumn::FeeRate => format!("{}", self.fee_rate),
        }
    }

    fn cmp(&self, other: &Self, column: LatestIncomingTxColumn) -> std::cmp::Ordering
    where
        Self: Sized,
    {
        match column {
            LatestIncomingTxColumn::TxHash => self.tx_hash.cmp(&other.tx_hash),
            LatestIncomingTxColumn::Time => self.time.cmp(&other.time).reverse(),
            LatestIncomingTxColumn::SizeInBytes => self.size_in_bytes.cmp(&other.size_in_bytes),
            LatestIncomingTxColumn::FeeRate => self.fee_rate.cmp(&other.fee_rate),
        }
    }
}

pub fn mempool_dashboard(_event_sender: mpsc::Sender<TUIEvent>) -> impl IntoBoxedView + use<> {
    LinearLayout::vertical()
        .child(
            LinearLayout::horizontal()
                .child(
                    Panel::new(
                        LinearLayout::vertical()
                            .child(TextView::new("[Transaction Distribution]"))
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• Total Pool size:").min_width(20))
                                    .child(TextView::empty().with_name(TOTAL_POOL_SIZE)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("  🟡 Pending:").min_width(20))
                                    .child(TextView::empty().with_name(PENDING)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("  🔵 Proposed:").min_width(20))
                                    .child(TextView::empty().with_name(PROPOSED)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("  🟢 Committing:").min_width(20))
                                    .child(TextView::empty().with_name(COMMITTING)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• Avg. Fee Rate:").min_width(20))
                                    .child(TextView::empty().with_name(AVG_FEE_RATE)),
                            ),
                    )
                    .min_width(50),
                )
                .child(
                    Panel::new(
                        LinearLayout::vertical()
                            .child(TextView::new("[Throughput & Trends]"))
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• Tx In").min_width(20))
                                    .child(TextView::empty().with_name(TX_IN)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• Tx Out:").min_width(20))
                                    .child(TextView::empty().with_name(TX_OUT)),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(TextView::new("• Avg. Block Time:").min_width(20))
                                    .child(TextView::empty().with_name(AVG_BLOCK_TIME)),
                            ),
                    )
                    .min_width(50),
                ),
        )
        .child(
            Panel::new(
                LinearLayout::vertical()
                    .child(TextView::new("[Rejections - Session]"))
                    .child(
                        LinearLayout::horizontal()
                            .child(TextView::new("• Total Rejections:").min_width(20))
                            .child(TextView::empty().with_name(TOTAL_REJECTION)),
                    )
                    .child(
                        LinearLayout::horizontal()
                            .child(TextView::new("• Rejection Rate:").min_width(20))
                            .child(TextView::empty().with_name(REJECTION_RATE)),
                    )
                    .child(TextView::new(" ").with_name(SUBSCRIBE_WARNING))
                    .child(
                        TableView::<RejectionItem, RejectionColumn>::new()
                            .column(RejectionColumn::Reason, "Rejection Reason", |c| c)
                            .column(RejectionColumn::Count, "Count", |c| c)
                            .default_column(RejectionColumn::Count)
                            .with_name(REJECTION_TABLE)
                            .min_size((50, 5)),
                    ),
            )
            .scrollable(),
        )
        .child(
            Panel::new(
                LinearLayout::vertical()
                    .child(TextView::new("[Latest Incoming Transactions]"))
                    .child(TextView::new(" "))
                    .child(
                        TableView::<LatestIncomingTxItem, LatestIncomingTxColumn>::new()
                            .column(LatestIncomingTxColumn::TxHash, "Tx Hash", |c| c)
                            .column(LatestIncomingTxColumn::Time, "Time", |c| c)
                            .column(LatestIncomingTxColumn::SizeInBytes, "Size (Bytes)", |c| c)
                            .column(
                                LatestIncomingTxColumn::FeeRate,
                                "Fee Rate (shannons/kB)",
                                |c| c,
                            )
                            .default_column(LatestIncomingTxColumn::Time)
                            .with_name(LATEST_INCOMING_TX_TABLE)
                            .min_size((50, 5)),
                    ),
            )
            .scrollable(),
        )
}

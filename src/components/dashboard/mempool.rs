use anyhow::Context;
use anyhow::anyhow;
use chrono::Local;
use cursive::{
    view::{IntoBoxedView, Nameable, Resizable},
    views::{LinearLayout, Panel, TextView},
};
use cursive_table_view::{TableView, TableViewItem};

use crate::{
    CURRENT_TAB,
    components::{
        DashboardData, UpdateToView,
        dashboard::mempool::names::{
            AVG_BLOCK_TIME, AVG_FEE_RATE, COMMITING, LATEST_INCOMING_TX_TABLE, PENDING, PROPOSED,
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
    COMMITING,
    AVG_FEE_RATE,
    TX_IN,
    TX_OUT,
    AVG_BLOCK_TIME,
    TOTAL_REJECTION,
    REJECTION_RATE,
    REJECTION_TABLE,
    LATEST_INCOMING_TX_TABLE
);
pub struct MempoolDashboardData {
    total_pool_size_in_tx: u64,
    total_pool_size_in_bytes: usize,
    pending_tx: u64,
    proposed_tx: u64,
    commiting_tx: u64,
    avg_fee_rate: u64,
    tx_in: usize,
    tx_out: usize,
    avg_block_time: f64,
    total_rejection: usize,
    rejection_rate: f64,
    rejection_details: Vec<RejectionItem>,
    latest_incoming_txs: Vec<LatestIncomingTxItem>,
}

impl DashboardData for MempoolDashboardData {
    fn fetch_data_through_client(client: &ckb_sdk::CkbRpcClient) -> anyhow::Result<Self> {
        let tx_pool_info = client
            .tx_pool_info()
            .with_context(|| anyhow!("Unable to get tx pool info"))?;
        let fee_rate_statistics = client
            .get_fee_rate_statistics(None)
            .with_context(|| anyhow!("Unable to get fee rate statistics"))?;
        Ok(Self {
            total_pool_size_in_tx: tx_pool_info.total_tx_size.value(),
            total_pool_size_in_bytes: 0,
            pending_tx: tx_pool_info.pending.value(),
            proposed_tx: tx_pool_info.proposed.value(),
            commiting_tx: 0,
            avg_fee_rate: fee_rate_statistics.unwrap().mean.value(),
            tx_in: 0,
            tx_out: 0,
            avg_block_time: -1.0,
            total_rejection: 0,
            rejection_rate: -1.0,
            rejection_details: vec![RejectionItem {
                count: 1,
                reason: format!("test"),
            }],
            latest_incoming_txs: vec![LatestIncomingTxItem {
                fee_rate: 1111,
                size_in_bytes: 2222,
                time: chrono::Local::now(),
                tx_hash: "111111".to_string(),
            }],
        })
    }
    fn should_update(&self) -> bool {
        CURRENT_TAB.load(std::sync::atomic::Ordering::SeqCst) == 2
    }
}

impl UpdateToView for MempoolDashboardData {
    fn update_to_view(&self, siv: &mut cursive::Cursive) {
        update_text!(
            siv,
            TOTAL_POOL_SIZE,
            format!(
                "{} txs ({:.1} MB)",
                self.total_pool_size_in_tx,
                self.total_pool_size_in_bytes as f64 / 1024.0 / 1024.0
            )
        );
        update_text!(siv, PENDING, format!("{}", self.pending_tx));
        update_text!(siv, PROPOSED, format!("{}", self.proposed_tx));
        update_text!(siv, COMMITING, format!("{}", self.commiting_tx));
        update_text!(
            siv,
            AVG_FEE_RATE,
            format!("{} shannons/KB", self.avg_fee_rate)
        );
        update_text!(siv, TX_IN, format!("{} tx/s", self.tx_in));
        update_text!(siv, TX_OUT, format!("{} tx/s", self.tx_out));
        update_text!(siv, AVG_BLOCK_TIME, format!("{:.1}s", self.avg_block_time));
        update_text!(siv, TOTAL_REJECTION, format!("{}", self.total_rejection));
        update_text!(
            siv,
            REJECTION_RATE,
            format!("{:.1}%", self.rejection_rate * 100.0)
        );
        siv.call_on_name(
            REJECTION_TABLE,
            |v: &mut TableView<RejectionItem, RejectionColumn>| {
                v.clear();
                for item in self.rejection_details.iter() {
                    v.insert_item(item.clone());
                }
            },
        );
        siv.call_on_name(
            LATEST_INCOMING_TX_TABLE,
            |v: &mut TableView<LatestIncomingTxItem, LatestIncomingTxColumn>| {
                v.clear();
                for item in self.latest_incoming_txs.iter() {
                    v.insert_item(item.clone());
                }
            },
        );
    }
}
#[derive(Clone)]
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
    size_in_bytes: usize,
    fee_rate: usize,
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
            LatestIncomingTxColumn::Time => self.time.cmp(&other.time),
            LatestIncomingTxColumn::SizeInBytes => self.size_in_bytes.cmp(&other.size_in_bytes),
            LatestIncomingTxColumn::FeeRate => self.fee_rate.cmp(&other.fee_rate),
        }
    }
}

pub fn mempool_dashboard() -> impl IntoBoxedView + use<> {
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
                                    .child(TextView::empty().with_name(COMMITING)),
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
        .child(Panel::new(
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
                .child(
                    TableView::<RejectionItem, RejectionColumn>::new()
                        .column(RejectionColumn::Reason, "Rejection Reason", |c| c)
                        .column(RejectionColumn::Count, "Count", |c| c)
                        .with_name(REJECTION_TABLE)
                        .min_size((50, 20)),
                ),
        ))
        .child(Panel::new(
            LinearLayout::vertical()
                .child(TextView::new("[Latest Incoming Transactions]"))
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
                        .with_name(LATEST_INCOMING_TX_TABLE)
                        .min_size((50, 20)),
                ),
        ))
}

use cursive::{
    view::{IntoBoxedView, Nameable, Resizable},
    views::{LinearLayout, Panel, TextView},
};

use crate::components::dashboard::mempool::names::{
    AVG_BLOCK_TIME, AVG_FEE_RATE, COMMITING, PENDING, PROPOSED, TOTAL_POOL_SIZE, TX_IN, TX_OUT,
};

mod names {
    pub const TOTAL_POOL_SIZE: &str = "mempool_dashboard_total_pool_size";
    pub const PENDING: &str = "mempool_dashboard_pending";
    pub const PROPOSED: &str = "mempool_dashboard_proposed";
    pub const COMMITING: &str = "mempool_dashboard_commiting";
    pub const AVG_FEE_RATE: &str = "mempool_dashboard_avg_fee_rate";
    pub const TX_IN: &str = "mempool_dashboard_tx_in";
    pub const TX_OUT: &str = "mempool_dashboard_tx_out";
    pub const AVG_BLOCK_TIME: &str = "mempool_dashboard_avg_block_time";
}

pub fn mempool_dashboard() -> impl IntoBoxedView + use<> {
    LinearLayout::vertical().child(
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
}

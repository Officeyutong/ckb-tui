use anyhow::{Context, anyhow};
use cursive::{
    view::{IntoBoxedView, Nameable, Resizable},
    views::{LinearLayout, Panel, TextView},
};

use crate::components::{
    FetchData, UpdateToView,
    dashboard::blockchain::names::{
        ALGORITHM, AVERAGE_BLOCK_TIME, BLOCK_HEIGHT, DIFFICULTY, EPOCH, ESTIMATED_EPOCH_TIME,
        HASH_RATE,
    },
    extract_epoch,
};

mod names {
    pub const EPOCH: &str = "blockchain_dashboard_epoch";
    pub const ESTIMATED_EPOCH_TIME: &str = "blockchain_dashboard_estimated_epoch_time";
    pub const BLOCK_HEIGHT: &str = "blockchain_dashboard_block_height";
    pub const AVERAGE_BLOCK_TIME: &str = "blockchain_dashboard_estimated_epoch_time";
    pub const ALGORITHM: &str = "blockchain_dashboard_algorithm";
    pub const DIFFICULTY: &str = "blockchain_dashboard_difficulty";
    pub const HASH_RATE: &str = "blockchain_dashboard_hash_rate";
}

pub struct BlockchainDashboardState {}

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

impl FetchData for BlockchainDashboardData {
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
    LinearLayout::vertical().child(
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
}

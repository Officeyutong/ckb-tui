use std::sync::mpsc;

use anyhow::{Context, anyhow};
use ckb_sdk::CkbRpcClient;
use cursive::{
    theme::{BaseColor, ColorStyle},
    utils::markup::StyledString,
    view::{IntoBoxedView, Nameable, Resizable},
    views::{LinearLayout, Panel, TextView},
};
use cursive_table_view::{TableView, TableViewItem};

use crate::{
    CURRENT_TAB,
    components::{
        DashboardData, UpdateToView,
        dashboard::{
            TUIEvent,
            peers::names::{AVG_LATENCY, CONNECTIONS, PEERS_TABLE, PUBLICLY_REACHABLE},
        },
    },
    declare_names, update_text,
};
declare_names!(
    names,
    "dashboard_peers_",
    PUBLICLY_REACHABLE,
    CONNECTIONS,
    AVG_LATENCY,
    PEERS_TABLE
);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum PeerDirection {
    In,
    Out,
}
#[derive(Clone)]
struct PeersItem {
    peer_id: String,
    direction: PeerDirection,
    block_height: Option<u64>,
    latency: u64,
    warning: Option<String>,
}
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum PeersColumn {
    PeerId,
    Direction,
    BlockHeight,
    Latency,
    Warning,
}

impl TableViewItem<PeersColumn> for PeersItem {
    fn to_column(&self, column: PeersColumn) -> String {
        match column {
            PeersColumn::PeerId => self.peer_id.clone(),
            PeersColumn::Direction => match self.direction {
                PeerDirection::In => String::from("In"),
                PeerDirection::Out => String::from("Out"),
            },
            PeersColumn::BlockHeight => match self.block_height {
                None => String::from("-"),
                Some(v) => format!("{}", v),
            },
            PeersColumn::Latency => format!("{}ms", self.latency),
            PeersColumn::Warning => match self.warning.clone() {
                None => String::from("-"),
                Some(v) => v,
            },
        }
    }

    fn cmp(&self, other: &Self, column: PeersColumn) -> std::cmp::Ordering
    where
        Self: Sized,
    {
        match column {
            PeersColumn::PeerId => self.peer_id.cmp(&other.peer_id),
            PeersColumn::Direction => self.direction.cmp(&other.direction),
            PeersColumn::BlockHeight => self.block_height.cmp(&other.block_height),
            PeersColumn::Latency => self.latency.cmp(&other.latency),
            PeersColumn::Warning => self.warning.cmp(&other.warning),
        }
    }
}
#[derive(Clone, Default)]
pub struct PeersDashboardData {
    connections_in: usize,
    connections_out: usize,
    peers: Vec<PeersItem>,
}

impl UpdateToView for PeersDashboardData {
    fn update_to_view(&self, siv: &mut cursive::Cursive) {
        let publicly_reachable = self
            .peers
            .iter()
            .any(|x| matches!(x.direction, PeerDirection::In));
        siv.call_on_name(PUBLICLY_REACHABLE, |s: &mut TextView| {
            let mut str = StyledString::new();

            if publicly_reachable {
                str.append_styled("✓ Yes", ColorStyle::front(BaseColor::Green));
            } else {
                str.append_styled("× No", ColorStyle::front(BaseColor::Red));
            }
            s.set_content(str);
        });
        update_text!(
            siv,
            CONNECTIONS,
            format!(
                "{} ({} outbound / {} inbound)",
                self.connections_in + self.connections_out,
                self.connections_out,
                self.connections_in
            )
        );
        let avg_latency =
            self.peers.iter().map(|x| x.latency).sum::<u64>() / self.peers.len() as u64;
        update_text!(siv, AVG_LATENCY, format!("{} ms", avg_latency));
        siv.call_on_name(PEERS_TABLE, |s: &mut TableView<PeersItem, PeersColumn>| {
            let selected_row = s.row();
            s.clear();
            for item in self.peers.iter() {
                s.insert_item(item.clone());
            }
            if let Some(row) = selected_row {
                if row < self.peers.len() {
                    s.set_selected_row(row);
                }
            }
        });
    }
}

impl DashboardData for PeersDashboardData {
    fn should_update(&self) -> bool {
        CURRENT_TAB.load(std::sync::atomic::Ordering::SeqCst) == 3
    }
    fn fetch_data_through_client(
        &mut self,
        client: &CkbRpcClient,
    ) -> anyhow::Result<Box<dyn DashboardData + Send + Sync>> {
        log::info!("Updating: PeersDashboardData");
        let peers = client
            .get_peers()
            .with_context(|| anyhow!("Unable to get peers"))?;
        let mut conn_in = 0;
        let mut conn_out = 0;
        peers.iter().for_each(|x| {
            if x.is_outbound {
                conn_out += 1;
            } else {
                conn_in += 1;
            }
        });
        *self = Self {
            connections_in: conn_in,
            connections_out: conn_out,
            peers: peers
                .into_iter()
                .map(|peer| PeersItem {
                    peer_id: peer.node_id,
                    direction: if peer.is_outbound {
                        PeerDirection::Out
                    } else {
                        PeerDirection::In
                    },
                    block_height: peer
                        .sync_state
                        .map(|x| x.best_known_header_number)
                        .flatten()
                        .map(|x| x.value()),
                    latency: 123,
                    warning: None,
                })
                .collect(),
        };
        log::info!("Updated: PeersDashboardData");
        Ok(Box::new(self.clone()))
    }
}

pub fn peers_dashboard(_event_sender: mpsc::Sender<TUIEvent>) -> impl IntoBoxedView + use<> {
    LinearLayout::vertical()
        .child(Panel::new(
            LinearLayout::vertical()
                .child(TextView::new("[Node Status]"))
                .child(
                    LinearLayout::horizontal()
                        .child(TextView::new("• Publicly Reachable?").min_width(25))
                        .child(TextView::empty().with_name(PUBLICLY_REACHABLE)),
                )
                .child(
                    LinearLayout::horizontal()
                        .child(TextView::new("• Connections:").min_width(25))
                        .child(TextView::empty().with_name(CONNECTIONS)),
                )
                .child(
                    LinearLayout::horizontal()
                        .child(TextView::new("• Avg. Latency:").min_width(25))
                        .child(TextView::empty().with_name(AVG_LATENCY)),
                ),
        ))
        .child(Panel::new(
            LinearLayout::vertical()
                .child(TextView::new("[Peers]"))
                .child(TextView::new(" "))
                .child(
                    TableView::<PeersItem, PeersColumn>::new()
                        .column(PeersColumn::PeerId, "Peer ID", |c| c)
                        .column(PeersColumn::Direction, "Direction", |c| c)
                        .column(PeersColumn::BlockHeight, "Block Height", |c| c)
                        .column(PeersColumn::Latency, "Latency", |c| c)
                        .column(PeersColumn::Warning, "Warning", |c| c)
                        .with_name(PEERS_TABLE)
                        .min_size((100, 10)),
                ),
        ))
}

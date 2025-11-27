use std::sync::{Arc, Mutex, atomic::AtomicBool, mpsc::TryRecvError};

use ckb_fixed_hash_core::H256;
use ckb_gen_types::core::ScriptHashType;
use ckb_jsonrpc_types::{JsonBytes, Script};
use ckb_sdk::{
    CkbRpcClient,
    rpc::ckb_indexer::{Cell, Pagination, SearchKey},
};
use cursive::{
    CbSink, Cursive, View,
    view::{IntoBoxedView, Nameable, Resizable},
    views::{Button, Dialog, LinearLayout, ListView, OnLayoutView, TextView},
};
use cursive_aligned_view::Alignable;
use cursive_async_view::{AsyncState, AsyncView};
use cursive_table_view::{TableView, TableViewItem};
use log::info;

use crate::{
    components::details::live_cells_searcher::display_cells_dialog::names::{
        CELLS_TABLE, PAGE_LABEL,
    },
    declare_names,
    utils::shorten_hex,
};

declare_names!(
    names,
    "live_cells_searcher_display_cells_dialog_",
    CELLS_TABLE,
    PAGE_LABEL
);

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum CellsDisplayColumns {
    BlockNumber,
    TxIndex,
    Capacity,
    OutPointTxHash,
    OutPointIndex,
}
#[derive(Clone, Debug)]
struct CellWrapper(Cell);

impl TableViewItem<CellsDisplayColumns> for CellWrapper {
    fn to_column(&self, column: CellsDisplayColumns) -> String {
        match column {
            CellsDisplayColumns::BlockNumber => self.0.block_number.value().to_string(),
            CellsDisplayColumns::TxIndex => self.0.tx_index.value().to_string(),
            CellsDisplayColumns::Capacity => {
                (self.0.output.capacity.value() as f64 / 1e8).to_string()
            }
            CellsDisplayColumns::OutPointTxHash => {
                shorten_hex(self.0.out_point.tx_hash.to_string(), 5, 5)
            }
            CellsDisplayColumns::OutPointIndex => self.0.out_point.index.value().to_string(),
        }
    }

    fn cmp(&self, other: &Self, column: CellsDisplayColumns) -> std::cmp::Ordering
    where
        Self: Sized,
    {
        match column {
            CellsDisplayColumns::BlockNumber => self
                .0
                .block_number
                .value()
                .cmp(&other.0.block_number.value()),
            CellsDisplayColumns::TxIndex => self.0.tx_index.value().cmp(&other.0.tx_index.value()),
            CellsDisplayColumns::Capacity => self
                .0
                .output
                .capacity
                .value()
                .cmp(&other.0.output.capacity.value()),
            CellsDisplayColumns::OutPointTxHash => {
                self.0.out_point.tx_hash.cmp(&other.0.out_point.tx_hash)
            }
            CellsDisplayColumns::OutPointIndex => self
                .0
                .out_point
                .index
                .value()
                .cmp(&other.0.out_point.index.value()),
        }
    }
}

struct CellsData {
    data: Vec<Pagination<Cell>>,
    current_page: usize,
    search_key: SearchKey,
    client: CkbRpcClient,
}

impl CellsData {
    pub fn new(search_key: SearchKey, client: CkbRpcClient) -> Self {
        Self {
            current_page: 0,
            data: Default::default(),
            search_key,
            client,
        }
    }
    pub fn switch_to_prev_page(data: Arc<Mutex<Self>>, siv: &mut Cursive) {
        let mut guard = data.lock().unwrap();

        if guard.current_page == 0 {
            siv.add_layer(
                Dialog::around(TextView::new("This is the first page")).button("Close", |siv| {
                    siv.pop_layer();
                }),
            );
            return;
        }
        guard.current_page -= 1;
        guard.update_data_to_view(siv.cb_sink().clone());
    }
    pub fn switch_to_next_page(data: Arc<Mutex<Self>>, siv: &mut Cursive) {
        info!("Switching to next page..");
        let mut guard = data.lock().unwrap();
        if guard.current_page + 1 == guard.data.len() {
            if guard
                .get_display_data()
                .map(|x| x.is_empty())
                .unwrap_or_default()
            {
                siv.add_layer(Dialog::around(TextView::new("No more data")).button(
                    "Close",
                    |siv| {
                        siv.pop_layer();
                    },
                ));
                return;
            }
            drop(guard);
            info!("Fetching with new thread..");
            Self::fetch_next_data_with_thread(data, Some(siv.cb_sink().clone()));
        } else {
            guard.current_page += 1;
            guard.update_data_to_view(siv.cb_sink().clone());
        }
    }
    pub fn get_display_data(&self) -> Option<&Vec<Cell>> {
        self.data.get(self.current_page).map(|x| &x.objects)
    }
    pub fn update_data_to_view(&self, cb_sink: CbSink) {
        if let Some(data) = self.get_display_data() {
            info!("Updating to view..");
            let data = data.clone();
            let page = self.current_page;
            cb_sink
                .send(Box::new(move |siv| {
                    siv.call_on_name(
                        CELLS_TABLE,
                        |view: &mut TableView<CellWrapper, CellsDisplayColumns>| {
                            info!("Setting items..");
                            view.set_items(data.into_iter().map(CellWrapper).collect());
                        },
                    );
                    siv.call_on_name(PAGE_LABEL, |view: &mut TextView| {
                        view.set_content(format!("Page {}", page + 1));
                    });
                }))
                .unwrap();
        }
    }
    pub fn fetch_next_data_with_thread(
        data: Arc<Mutex<Self>>,
        update_after_fetching: Option<CbSink>,
    ) -> std::sync::mpsc::Receiver<anyhow::Result<()>> {
        let (tx, rx) = std::sync::mpsc::sync_channel::<anyhow::Result<()>>(1);

        std::thread::spawn(move || {
            let mut guard = data.lock().unwrap();
            info!("Search keyword: {:?}", guard.search_key);
            match guard.client.get_cells(
                guard.search_key.clone(),
                ckb_sdk::rpc::ckb_indexer::Order::Desc,
                (18u32).into(),
                guard.data.last().map(|x| x.last_cursor.clone()),
            ) {
                Ok(o) => {
                    info!("Got data {:#?}", o.objects);
                    guard.data.push(o);
                    guard.current_page = guard.data.len() - 1;
                    if let Some(cb_sink) = update_after_fetching {
                        guard.update_data_to_view(cb_sink);
                    }
                    tx.send(Ok(())).ok();
                }
                Err(e) => {
                    tx.send(Err(e.into())).ok();
                }
            }
        });
        rx
    }
}
fn load_next_page(
    siv: &mut Cursive,
    data: Arc<Mutex<CellsData>>,
    update_to_view_after_loading: bool,
) {
    let rx = CellsData::fetch_next_data_with_thread(
        data,
        if update_to_view_after_loading {
            Some(siv.cb_sink().clone())
        } else {
            None
        },
    );
    let cb_sink = siv.cb_sink().clone();
    let async_view = AsyncView::new(siv, move || match rx.try_recv() {
        Ok(Ok(_)) => {
            cb_sink
                .send(Box::new(|siv| {
                    siv.pop_layer();
                }))
                .unwrap();
            AsyncState::Available(TextView::new("Loaded"))
        }
        Ok(Err(e)) => {
            cb_sink
                .send(Box::new(move |siv| {
                    siv.pop_layer();
                    siv.add_layer(Dialog::around(TextView::new(format!("{:?}", e))).button(
                        "Close",
                        |siv| {
                            siv.pop_layer();
                        },
                    ));
                }))
                .unwrap();
            AsyncState::Pending
        }
        Err(TryRecvError::Empty) => AsyncState::Pending,
        _ => AsyncState::Pending,
    });

    siv.add_layer(async_view);
}
pub fn display_cells_dialog(
    client: &CkbRpcClient,
    lock_args: JsonBytes,
    lock_hash: H256,
    script_hash_type: ScriptHashType,
    cb_sink: CbSink,
) -> impl IntoBoxedView {
    let data = Arc::new(Mutex::<CellsData>::new(CellsData::new(
        SearchKey {
            script: Script {
                args: lock_args,
                code_hash: lock_hash,
                hash_type: script_hash_type.into(),
            },
            script_type: ckb_sdk::rpc::ckb_indexer::ScriptType::Lock,
            filter: None,
            group_by_transaction: Some(false),
            script_search_mode: None,
            with_data: Some(false),
        },
        client.clone(),
    )));
    let initialized = Arc::new(AtomicBool::new(false));
    let initialized_cloned = initialized.clone();
    let data_cloned = data.clone();
    let data_cloned_2 = data.clone();
    let data_cloned_3 = data.clone();

    OnLayoutView::new(
        Dialog::new()
            .title("Live Cells")
            .content(
                LinearLayout::vertical()
                    .child(
                        TableView::<CellWrapper, CellsDisplayColumns>::new()
                            .column(CellsDisplayColumns::BlockNumber, "Block Number", |c| {
                                c.width(15)
                            })
                            .column(CellsDisplayColumns::TxIndex, "Transaction Index", |c| {
                                c.width(20)
                            })
                            .column(CellsDisplayColumns::Capacity, "Capacity (CKB)", |c| {
                                c.width(20)
                            })
                            .column(
                                CellsDisplayColumns::OutPointTxHash,
                                "OutPoint Tx Hash",
                                |c| c.width(20),
                            )
                            .column(CellsDisplayColumns::OutPointIndex, "OutPoint Index", |c| {
                                c.width(20)
                            })
                            .on_submit(|siv, _, data_index| {
                                let data = siv
                                    .call_on_name(
                                        CELLS_TABLE,
                                        |view: &mut TableView<CellWrapper, CellsDisplayColumns>| {
                                            view.borrow_items()[data_index].clone()
                                        },
                                    )
                                    .unwrap();
                                siv.add_layer(cell_detail_dialog(&data.0));
                            })
                            .with_name(CELLS_TABLE)
                            .min_width(110)
                            .min_height(20),
                    )
                    .child(
                        LinearLayout::horizontal()
                            .child(Button::new("Prev", move |siv| {
                                CellsData::switch_to_prev_page(data_cloned_2.clone(), siv);
                            }))
                            .child(
                                TextView::new("Page 1")
                                    .center()
                                    .with_name(PAGE_LABEL)
                                    .min_width(40),
                            )
                            .child(Button::new("Next", move |siv| {
                                CellsData::switch_to_next_page(data_cloned_3.clone(), siv);
                            }))
                            .align_center(),
                    ),
            )
            .button("Close", |siv| {
                siv.pop_layer();
            }),
        move |v, s| {
            v.layout(s);
            if !initialized_cloned.load(std::sync::atomic::Ordering::SeqCst) {
                let value = data_cloned.clone();
                cb_sink
                    .send(Box::new(move |siv| {
                        load_next_page(siv, value, true);
                    }))
                    .unwrap();
                initialized_cloned.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        },
    )
}

fn cell_detail_dialog(data: &Cell) -> impl IntoBoxedView {
    let mut list_view = ListView::new()
        .child(
            "Capacity (in shannons):",
            TextView::new(format!("{}", data.output.capacity.value())),
        )
        .child(
            "OutPoint Tx Hash:",
            TextView::new(data.out_point.tx_hash.to_string()),
        )
        .child(
            "OutPoint Index:",
            TextView::new(format!("{}", data.out_point.index.value())),
        )
        .child(
            "Block Number:",
            TextView::new(format!("{}", data.block_number.value())),
        )
        .child(
            "Tx Index:",
            TextView::new(format!("{}", data.tx_index.value())),
        )
        .child(
            "Lock Script Code Hash:",
            TextView::new(data.output.lock.code_hash.to_string()),
        )
        .child(
            "Lock Script Hash Type:",
            TextView::new(format!("{:?}", data.output.lock.hash_type)),
        )
        .child(
            "Lock Script Args:",
            TextView::new(format!(
                "0x{}",
                byteutils::bytes_to_hex(data.output.lock.args.as_bytes())
            )),
        );
    match &data.output.type_ {
        Some(Script {
            args,
            code_hash,
            hash_type,
        }) => {
            list_view.add_child(
                "Type Script Code Hash:",
                TextView::new(code_hash.to_string()),
            );
            list_view.add_child(
                "Type Script Hash Type:",
                TextView::new(format!("{:?}", hash_type)),
            );
            list_view.add_child(
                "Type Script Args:",
                TextView::new(format!("0x{}", byteutils::bytes_to_hex(args.as_bytes()))),
            );
        }
        None => list_view.add_child("Type Script:", TextView::new("N/A")),
    }
    Dialog::new()
        .title("Details of Cell")
        .button("Close", |siv| {
            siv.pop_layer();
        })
        .content(list_view)
}

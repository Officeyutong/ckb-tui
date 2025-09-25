use std::{
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

use ckb_sdk::CkbRpcClient;
use clap::Parser;
use cursive::{
    Cursive,
    view::Resizable,
    views::{Dialog, DummyView, TextView},
};
use cursive_async_view::AsyncView;

use crate::components::{
    DashboardData, DashboardState, UpdateToView,
    dashboard::{
        GeneralDashboardData,
        blockchain::{BlockchainDashboardData, BlockchainDashboardState},
        dashboard,
        overview::{OverviewDashboardData, OverviewDashboardState},
        set_loading,
    },
};

mod components;
mod utils;
enum SyncRequest {
    Stop,
    RequestSync { pop_layer_at_end: bool },
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// RPC endpoint of CKB node
    #[arg(short, long, default_value_t = String::from("https://testnet.ckb.dev/"))]
    rpc_url: String,
}

fn main() -> anyhow::Result<()> {
    cursive::logger::set_filter_levels_from_env();
    cursive::logger::init();
    let args = Args::parse();
    let client = CkbRpcClient::new(&args.rpc_url);
    let mut siv = cursive::default();
    siv.add_global_callback('q', |s| s.quit());
    siv.add_global_callback('~', cursive::Cursive::toggle_debug_console);
    let loading_variable = Arc::new(AtomicBool::new(false));

    let tx = {
        let (tx, rx) = std::sync::mpsc::channel::<SyncRequest>();
        let cb_sink = siv.cb_sink().clone();
        let loading_variable = loading_variable.clone();
        let client = client.clone();
        std::thread::spawn(move || {
            let client_cloned = client.clone();
            cb_sink
                .send(Box::new(move |siv| {
                    match GeneralDashboardData::fetch_data_through_client(&client_cloned) {
                        Ok(result) => {
                            result.update_to_view(siv);
                        }
                        Err(_) => {}
                    };
                    match BlockchainDashboardData::fetch_data_through_client(&client_cloned) {
                        Ok(result) => {
                            result.update_to_view(siv);
                        }
                        Err(_) => {}
                    }
                }))
                .unwrap();

            loop {
                match rx.recv().unwrap() {
                    SyncRequest::Stop => break,
                    SyncRequest::RequestSync { pop_layer_at_end } => {
                        loading_variable.store(true, std::sync::atomic::Ordering::SeqCst);
                        let data_basic = OverviewDashboardData::fetch_data_through_client(&client);
                        let data_blockchain =
                            BlockchainDashboardData::fetch_data_through_client(&client);
                        cb_sink
                            .send(Box::new(move |siv: &mut Cursive| {
                                if pop_layer_at_end {
                                    siv.pop_layer();
                                }

                                let result: anyhow::Result<(
                                    OverviewDashboardData,
                                    BlockchainDashboardData,
                                )> = (move || Ok((data_basic?, data_blockchain?)))();

                                match result {
                                    Ok((o1, o2)) => {
                                        o1.update_to_view(siv);
                                        o2.update_to_view(siv);
                                    }
                                    Err(err) => {
                                        siv.add_layer(
                                            Dialog::around(TextView::new(format!("{:?}", err)))
                                                .title("Error")
                                                .button("Close", |s| {
                                                    s.pop_layer();
                                                }),
                                        );
                                    }
                                }
                                set_loading(siv, false);
                            }))
                            .unwrap();
                        loading_variable.store(false, std::sync::atomic::Ordering::SeqCst);
                    }
                }
            }
        });
        tx
    };
    {
        let tx = tx.clone();
        siv.add_global_callback('r', move |siv| {
            if loading_variable.load(std::sync::atomic::Ordering::SeqCst) {
                return;
            }
            set_loading(siv, true);
            let content_view = Dialog::around(AsyncView::new(siv, || {
                cursive_async_view::AsyncState::<DummyView>::Pending
            }))
            .title("Refreshing..")
            .fixed_width(50);

            siv.add_layer(content_view);
            tx.send(SyncRequest::RequestSync {
                pop_layer_at_end: true,
            })
            .unwrap();
        });
    }
    {
        let tx = tx.clone();
        let cb_sink = siv.cb_sink().clone();
        std::thread::spawn(move || {
            let mut overview_state = OverviewDashboardState::new(client.clone());
            let mut blockchain_state = BlockchainDashboardState::new(client.clone());

            loop {
                cb_sink
                    .send(Box::new(|siv| set_loading(siv, true)))
                    .unwrap();
                let result = (|| {
                    anyhow::Ok((
                        overview_state.update_state()?,
                        blockchain_state.update_state()?,
                    ))
                })();
                match result {
                    Ok(_) => {}
                    Err(e) => {
                        cb_sink
                            .send(Box::new(move |siv| {
                                siv.add_layer(
                                    Dialog::around(TextView::new(format!(
                                        "Unable to update state: {:?}",
                                        e
                                    )))
                                    .button("Ok", |siv| {
                                        siv.pop_layer();
                                    }),
                                )
                            }))
                            .unwrap();
                    }
                };
                let overview_state = overview_state.clone();
                let blockchain_state = blockchain_state.clone();
                
                cb_sink
                    .send(Box::new(move |siv| {
                        overview_state.update_to_view(siv);
                        blockchain_state.update_to_view(siv);
                    }))
                    .unwrap();

                tx.send(SyncRequest::RequestSync {
                    pop_layer_at_end: false,
                })
                .unwrap();
                std::thread::sleep(Duration::from_secs(1));
            }
        });
    }
    tx.send(SyncRequest::RequestSync {
        pop_layer_at_end: false,
    })
    .unwrap();
    siv.set_autorefresh(true);
    siv.add_layer(dashboard());
    siv.run();
    tx.send(SyncRequest::Stop).unwrap();
    Ok(())
}

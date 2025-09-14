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
    FetchData, UpdateState, UpdateToView,
    dashboard::{
        GeneralDashboardData, dashboard,
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
    #[arg(short, long, default_value_t = String::from("http://127.0.0.1:8114"))]
    rpc_url: String,
}

fn main() -> anyhow::Result<()> {
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
                .send(Box::new(
                    move |siv| match GeneralDashboardData::fetch_data_through_client(&client_cloned)
                    {
                        Ok(result) => {
                            result.update_to_view(siv);
                        }
                        Err(_) => {}
                    },
                ))
                .unwrap();

            loop {
                match rx.recv().unwrap() {
                    SyncRequest::Stop => break,
                    SyncRequest::RequestSync { pop_layer_at_end } => {
                        loading_variable.store(true, std::sync::atomic::Ordering::SeqCst);
                        let data = OverviewDashboardData::fetch_data_through_client(&client);

                        cb_sink
                            .send(Box::new(move |siv: &mut Cursive| {
                                if pop_layer_at_end {
                                    siv.pop_layer();
                                }
                                match data {
                                    Ok(o) => o.update_to_view(siv),
                                    Err(err) => {
                                        siv.add_layer(
                                            Dialog::around(TextView::new(format!("{}", err)))
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
            let mut overview_state = OverviewDashboardState::default();
            loop {
                cb_sink
                    .send(Box::new(|siv| set_loading(siv, true)))
                    .unwrap();
                overview_state = overview_state.update_state();
                let overview_state = overview_state.clone();
                cb_sink
                    .send(Box::new(move |siv| overview_state.update_to_view(siv)))
                    .unwrap();

                tx.send(SyncRequest::RequestSync {
                    pop_layer_at_end: false,
                })
                .unwrap();
                std::thread::sleep(Duration::from_secs(3));
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

use std::sync::{Arc, atomic::AtomicBool};

use ckb_sdk::CkbRpcClient;
use clap::Parser;
use cursive::{
    Cursive,
    view::Resizable,
    views::{Dialog, DummyView, TextView},
};
use cursive_async_view::AsyncView;

use crate::components::dashboard::{dashboard, overview::fetch_overview_data};

mod components;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// RPC endpoint of CKB node
    #[arg(short, long, default_value_t=String::from("http://127.0.0.1:8114"))]
    rpc_url: String,
}

fn main() -> anyhow::Result<()> {
    cursive::logger::init();
    let args = Args::parse();
    let client = CkbRpcClient::new(&args.rpc_url);
    let mut cur = cursive::default();
    cur.add_global_callback('q', |s| s.quit());
    cur.add_global_callback('~', cursive::Cursive::toggle_debug_console);
    let loading_variable = Arc::new(AtomicBool::new(false));

    cur.add_global_callback('r', move |cur| {
        if loading_variable.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        let cb_sink = cur.cb_sink().clone();
        let loading_variable = loading_variable.clone();
        let client = client.clone();
        let content_view = Dialog::around(AsyncView::new(cur, || {
            cursive_async_view::AsyncState::<DummyView>::Pending
        }))
        .title("Refreshing..")
        .fixed_width(50);

        cur.add_layer(content_view);
        std::thread::spawn(move || {
            loading_variable.store(true, std::sync::atomic::Ordering::SeqCst);
            let data = fetch_overview_data(&client);

            cb_sink
                .send(Box::new(move |siv: &mut Cursive| {
                    siv.pop_layer();
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
                }))
                .unwrap();
            loading_variable.store(false, std::sync::atomic::Ordering::SeqCst);
        });
    });

    cur.add_layer(dashboard());
    cur.run();

    Ok(())
}

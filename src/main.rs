use std::time::Duration;

use ckb_sdk::CkbRpcClient;
use clap::Parser;
use cursive::Cursive;
use rand::Rng;

use crate::components::dashboard::{dashboard, overview::OverviewDashboardData};

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
    cur.add_layer(dashboard());
    let cb_sink = cur.cb_sink().clone();
    std::thread::spawn(move || {
        let mut rng = rand::rng();
        loop {
            let data = OverviewDashboardData {
                average_latency: rng.random_range(1..1000),
                current_block: rng.random_range(1..1000),
                estimated_time_left: rng.random_range(1..1000),
                inbound_peers: rng.random_range(1..10),
                outbound_peers: rng.random_range(1..10),
                syncing_progress: rng.random(),
                cpu_percent: rng.random(),
                disk_total: rng.random_range(1..1000),
                disk_used: rng.random_range(1..1000),
                last_coming_tx: rng.random_range(1..1000),
                ram_total: rng.random_range(1..1000),
                ram_used: rng.random_range(1..1000),
                relaying_count: rng.random_range(1..1000),
                total_nodes_online: rng.random_range(1..1000),
                tx_pool_pending: rng.random_range(1..1000),
            };
            cb_sink
                .send(Box::new(move |siv: &mut Cursive| {
                    data.update_to_view("root", siv);
                }))
                .unwrap();
            std::thread::sleep(Duration::from_secs(1));
        }
    });
    cur.run();

    Ok(())
}

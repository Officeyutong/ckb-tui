use ckb_tui::start_ckb_tui;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// RPC endpoint of CKB node
    #[arg(short = 'r', long, default_value_t = String::from("http://127.0.0.1:8114"))]
    rpc_url: String,
    /// TCP endpoint of CKB node, used for receiving pushed transactions data
    /// If not provided, latest transactions and rejected transactions won't be displayed
    #[arg(short, long)]
    tcp_url: Option<String>,
    /// Refresh interval of displayed data, defaults to 300ms
    #[arg(short = 'i', long, default_value_t = 300)]
    refresh_interval: usize,

    /// Theme file to use for cursive. See https://github.com/gyscos/cursive/blob/main/cursive/examples/assets/style.toml for an example.
    #[arg(long)]
    theme_file: Option<String>,
}
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    start_ckb_tui(
        &args.rpc_url,
        args.tcp_url,
        args.refresh_interval,
        args.theme_file,
    )?;

    Ok(())
}

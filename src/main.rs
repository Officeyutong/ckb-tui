use ckb_tui::start_ckb_tui;

fn main() -> anyhow::Result<()> {
    start_ckb_tui(&std::env::args().collect::<Vec<_>>())?;
    Ok(())
}

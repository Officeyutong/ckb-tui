use number_prefix::NumberPrefix;
use tokio::net::TcpStream;

pub mod bar_chart;

#[macro_export]
macro_rules! update_text {
    ($siv:expr, $name:expr, $content:expr) => {
        $siv.call_on_name($name, |v: &mut cursive::views::TextView| {
            v.set_content($content)
        })
    };
}

#[macro_export]
macro_rules! declare_names {
    ($module_name:ident,$prefix:literal, $($variable:ident),*) => {
        mod $module_name {
            $(
                pub const $variable: &str = concat!($prefix, stringify!($variable));
            )*
        }
    };
}

pub fn shorten_hex(hex: impl AsRef<str>, keep_prefix: usize, keep_suffix: usize) -> String {
    let hex = hex.as_ref();
    if hex.len() <= keep_prefix + keep_suffix {
        return hex.to_string();
    }
    let prefix = if hex.starts_with("0x") { "" } else { "0x" };
    format!(
        "{}{}...{}",
        prefix,
        &hex[..keep_prefix],
        &hex[hex.len() - keep_suffix..]
    )
}

pub fn hash_rate_to_string(hash_rate: f64) -> String {
    match NumberPrefix::decimal(hash_rate) {
        NumberPrefix::Standalone(s) => format!("{} H/s", s),
        NumberPrefix::Prefixed(prefix, n) => format!("{:.2} {}H/s", n, prefix),
    }
}

pub fn difficulty_to_string(difficulty: f64) -> String {
    match NumberPrefix::decimal(difficulty) {
        NumberPrefix::Standalone(s) => format!("{} H", s),
        NumberPrefix::Prefixed(prefix, n) => format!("{:.2} {}H", n, prefix),
    }
}

pub async fn create_subscription_client(
    addr: &str,
) -> anyhow::Result<ckb_sdk::pubsub::Client<TcpStream>> {
    log::debug!("Connecting TCP: {}", addr);
    Ok(ckb_sdk::pubsub::Client::new(
        TcpStream::connect(addr).await?,
    ))
}

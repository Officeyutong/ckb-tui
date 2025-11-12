# CKB-TUI Usage Guide

## Build and Preparation

- It is recommended to use the CKB from this PR https://github.com/nervosnetwork/ckb/pull/4989, otherwise some data will be missing (displayed as N/A)
    - If using the CKB from PR #4989, you need to add the Terminal module to rpc.modules in ckb.toml
- Get this repo

## Usage

```bash
cargo run -- -r http://192.168.15.189:8114 -t 192.168.15.189:18114
```

- The `-r` parameter specifies the JSON RPC service address provided by the CKB node. If not provided, defaults to `http://127.0.0.1:8114`
- The `-t` parameter specifies the TCP service address provided by the CKB node. If not provided, recent new transactions/recent rejected transactions will not display data. This data depends on the CKB node's TCP streaming.
    - CKB does not listen on TCP service by default. If you need to enable it, you must uncomment `rpc.tcp_listen_address`
- The TUI will automatically refresh after startup.
- While the TUI is running, press "Shift + `" to open the log window
- While the TUI is running, press Tab to switch focus, press Enter to confirm

## Known Issues

- Some data (such as parts related to node network latency, and logs) are currently unavailable and show dummy data


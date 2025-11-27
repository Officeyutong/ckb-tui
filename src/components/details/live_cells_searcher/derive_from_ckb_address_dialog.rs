use std::str::FromStr;

use anyhow::{Context, anyhow, bail};
use ckb_gen_types::core::ScriptHashType;
use ckb_sdk::Address;
use cursive::{
    CbSink, Cursive,
    view::{IntoBoxedView, Nameable},
    views::{Button, Dialog, EditView, LinearLayout, Panel, RadioButton, RadioGroup, TextView},
};
use cursive_spinner_view::SpinnerView;
use log::info;
use serde::Deserialize;

use crate::{
    components::details::live_cells_searcher::derive_from_ckb_address_dialog::names::{
        ADDRESS_INPUT, CKB_CLI_ACCOUNT_ENTRY, CKB_CLI_ACCOUNTS, CKB_CLI_ACCOUNTS_VIEW,
        LOAD_CKB_CLI_ACCOUNT, LOAD_CKB_CLI_ACCOUNT_SPINNER,
    },
    declare_names,
};

declare_names!(
    names,
    "live_cells_searcher_derive_from_ckb_address_dialog_",
    ADDRESS_INPUT,
    LOAD_CKB_CLI_ACCOUNT,
    LOAD_CKB_CLI_ACCOUNT_SPINNER,
    CKB_CLI_ACCOUNTS_VIEW,
    CKB_CLI_ACCOUNTS,
    CKB_CLI_ACCOUNT_ENTRY
);

#[derive(Deserialize)]
struct CkbCliAccount {
    address: CkbCliAccountAddress,
}
#[derive(Deserialize)]
struct CkbCliAccountAddress {
    mainnet: String,
    testnet: String,
}

fn load_ckb_cli_account(siv: &mut Cursive) {
    siv.call_on_name(LOAD_CKB_CLI_ACCOUNT_SPINNER, |view: &mut SpinnerView| {
        view.spin_up();
    });
    let cb_sink = siv.cb_sink().clone();
    std::thread::spawn(move || {
        let output = std::process::Command::new("ckb-cli")
            .arg("account")
            .arg("list")
            .output();
        cb_sink
            .send(Box::new(|siv| {
                siv.call_on_name(LOAD_CKB_CLI_ACCOUNT_SPINNER, |view: &mut SpinnerView| {
                    view.spin_down();
                });
            }))
            .unwrap();
        let accounts: Result<Vec<CkbCliAccount>, anyhow::Error> = (move || {
            let output = output.with_context(|| anyhow!("Unable to run ckb-cli account list"))?;
            if !output.status.success() {
                bail!(
                    "Unable to execute: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            } else {
                Ok(serde_yaml::from_slice::<Vec<CkbCliAccount>>(&output.stdout)
                    .with_context(|| anyhow!("Unable to deserialize output"))?)
            }
        })();
        let accounts = match accounts {
            Ok(o) => o,
            Err(e) => {
                cb_sink
                    .send(Box::new(move |siv| {
                        siv.add_layer(Dialog::around(TextView::new(format!("{:?}", e))).button(
                            "Close",
                            |siv| {
                                siv.pop_layer();
                            },
                        ));
                    }))
                    .unwrap();
                return;
            }
        };
        cb_sink
            .send(Box::new(move |siv| {
                siv.call_on_name(CKB_CLI_ACCOUNTS_VIEW, |view: &mut LinearLayout| {
                    view.clear();
                    for item in accounts.into_iter() {
                        view.add_child(
                            RadioButton::global_str(CKB_CLI_ACCOUNTS, item.address.mainnet)
                                .with_name(CKB_CLI_ACCOUNT_ENTRY),
                        );
                        view.add_child(
                            RadioButton::global_str(CKB_CLI_ACCOUNTS, item.address.testnet)
                                .with_name(CKB_CLI_ACCOUNT_ENTRY),
                        );
                    }
                });
            }))
            .unwrap();
    });
}

pub fn derive_from_address_dialog(
    callback: impl Fn(String, String, ScriptHashType) + Send + Sync + 'static,
    cb_sink: CbSink,
) -> impl IntoBoxedView {
    let mut choice_group = RadioGroup::<String>::new();
    choice_group.set_on_change(|siv, text| {
        match text.as_str() {
            "Input" => {
                siv.call_on_name(LOAD_CKB_CLI_ACCOUNT, |view: &mut Button| {
                    view.disable();
                });
                siv.call_on_all_named(CKB_CLI_ACCOUNT_ENTRY, |view: &mut RadioButton<String>| {
                    view.disable();
                });
                siv.call_on_name(ADDRESS_INPUT, |view: &mut EditView| {
                    view.enable();
                });
            }
            "Select from ckb-cli accounts" => {
                siv.call_on_name(LOAD_CKB_CLI_ACCOUNT, |view: &mut Button| {
                    view.enable();
                });
                siv.call_on_all_named(CKB_CLI_ACCOUNT_ENTRY, |view: &mut RadioButton<String>| {
                    view.enable();
                });
                siv.call_on_name(ADDRESS_INPUT, |view: &mut EditView| {
                    view.disable();
                });
            }
            _ => unreachable!(),
        };
    });
    Dialog::new()
        .title("Derive from CKB address")
        .content(
            LinearLayout::vertical()
                .child(choice_group.button_str("Input").selected())
                .child(Panel::new(
                    LinearLayout::vertical().child(EditView::new().with_name(ADDRESS_INPUT)),
                ))
                .child(choice_group.button_str("Select from ckb-cli accounts"))
                .child(Panel::new(
                    LinearLayout::vertical()
                        .child(
                            Button::new("Load", load_ckb_cli_account)
                                .disabled()
                                .with_name(LOAD_CKB_CLI_ACCOUNT),
                        )
                        .child(SpinnerView::new(cb_sink).with_name(LOAD_CKB_CLI_ACCOUNT_SPINNER))
                        .child(LinearLayout::vertical().with_name(CKB_CLI_ACCOUNTS_VIEW)),
                )),
        )
        .button("Confirm", move |siv| {
            let ckb_address = match choice_group.selection().as_str() {
                "Input" => siv
                    .call_on_name(ADDRESS_INPUT, |view: &mut EditView| {
                        view.get_content().to_string()
                    })
                    .unwrap(),
                "Select from ckb-cli accounts" => {
                    RadioGroup::<String>::with_global(CKB_CLI_ACCOUNTS, |group| {
                        group.selection().to_string()
                    })
                }
                _ => unreachable!(),
            };
            info!(
                "Using ckb address {} for deriving args and hash..",
                ckb_address
            );
            match Address::from_str(&ckb_address) {
                Ok(o) => {
                    let payload = o.payload();
                    let mut args_output = String::from("0x");
                    for item in payload.args().iter() {
                        args_output.push_str(&format!("{:02x}", item));
                    }
                    let mut hash_output = String::from("0x");

                    for item in payload.code_hash(None).raw_data().iter() {
                        hash_output.push_str(&format!("{:02x}", item));
                    }
                    if !matches!(
                        payload.hash_type(),
                        ScriptHashType::Data
                            | ScriptHashType::Data1
                            | ScriptHashType::Data2
                            | ScriptHashType::Type
                    ) {
                        siv.add_layer(
                            Dialog::around(TextView::new(
                                "Only supports ScriptHashType of Type, Data, Data1, Data2",
                            ))
                            .title("Error")
                            .button("Close", |siv| {
                                siv.pop_layer();
                            }),
                        );
                        return;
                    }
                    callback(args_output, hash_output, payload.hash_type());
                    siv.pop_layer();
                }
                Err(e) => {
                    siv.add_layer(
                        Dialog::around(TextView::new(format!(
                            "Bad address: {}\n{}",
                            ckb_address, e
                        )))
                        .title("Error")
                        .button("Close", |siv| {
                            siv.pop_layer();
                        }),
                    );
                }
            }
        })
        .button("Cancel", |siv| {
            siv.pop_layer();
        })
}

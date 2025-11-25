mod derive_from_ckb_address_dialog;
mod display_cells_dialog;
use std::str::FromStr;

use crate::{
    components::details::live_cells_searcher::{
        derive_from_ckb_address_dialog::derive_from_address_dialog,
        names::{
            HASH_TYPE_RADIO_DATA, HASH_TYPE_RADIO_DATA1, HASH_TYPE_RADIO_DATA2,
            HASH_TYPE_RADIO_TYPE, LOCK_ARGS, LOCK_HASH,
        },
    },
    declare_names,
};
use anyhow::{anyhow, bail, Context};
use ckb_fixed_hash_core::H256;
use ckb_gen_types::core::ScriptHashType;
use ckb_sdk::CkbRpcClient;
use cursive::{
    view::{IntoBoxedView, Nameable, Resizable},
    views::{
        Button, Dialog, DummyView, EditView, LinearLayout, ListView, RadioButton, RadioGroup,
        TextView,
    },
};
use cursive_aligned_view::Alignable;
use display_cells_dialog::display_cells_dialog;
use serde_json::json;

declare_names!(
    names,
    "live_cells_searcher_",
    LOCK_ARGS,
    LOCK_HASH,
    HASH_TYPE_RADIO_TYPE,
    HASH_TYPE_RADIO_DATA,
    HASH_TYPE_RADIO_DATA1,
    HASH_TYPE_RADIO_DATA2
);

pub fn live_cells_searcher(client: &CkbRpcClient) -> impl IntoBoxedView {
    let client_cloned = client.clone();
    let mut script_hash_type_radios = RadioGroup::<ScriptHashType>::new();
    Dialog::new()
        .title("Live Cells Searcher")
        .content(
            LinearLayout::vertical()
                .child(
                    ListView::new()
                        .child(
                            "Lock Args:",
                            EditView::new().with_name(LOCK_ARGS).min_width(50),
                        )
                        .child(" ", DummyView::new())
                        .child(
                            "Lock Hash:",
                            EditView::new().with_name(LOCK_HASH).min_width(50),
                        )
                        .child(" ", DummyView::new())
                        .child(
                            "Script Hash Type:",
                            LinearLayout::horizontal()
                                .child(
                                    script_hash_type_radios
                                        .button(ScriptHashType::Type, "Type")
                                        .with_name(HASH_TYPE_RADIO_TYPE),
                                )
                                .child(
                                    script_hash_type_radios
                                        .button(ScriptHashType::Data, "Data")
                                        .with_name(HASH_TYPE_RADIO_DATA),
                                )
                                .child(
                                    script_hash_type_radios
                                        .button(ScriptHashType::Data1, "Data1")
                                        .with_name(HASH_TYPE_RADIO_DATA1),
                                )
                                .child(
                                    script_hash_type_radios
                                        .button(ScriptHashType::Data2, "Data2")
                                        .with_name(HASH_TYPE_RADIO_DATA2),
                                ),
                        )
                        .min_width(50),
                )
                .child(DummyView::new())
                .child(
                    Button::new("Derive from CKB address", move |siv| {
                        let cb_sink = siv.cb_sink().clone();
                        let cb_sink_2 = siv.cb_sink().clone();

                        siv.add_layer(derive_from_address_dialog(
                            move |lock_args, lock_hash, script_hash_type| {
                                cb_sink
                                    .send(Box::new(move |siv| {
                                        siv.call_on_name(LOCK_ARGS, |view: &mut EditView| {
                                            view.set_content(lock_args)
                                        });
                                        siv.call_on_name(LOCK_HASH, |view: &mut EditView| {
                                            view.set_content(lock_hash)
                                        });
                                        siv.call_on_name(
                                            match script_hash_type {
                                                ScriptHashType::Type => HASH_TYPE_RADIO_TYPE,
                                                ScriptHashType::Data => HASH_TYPE_RADIO_DATA,
                                                ScriptHashType::Data1 => HASH_TYPE_RADIO_DATA1,
                                                ScriptHashType::Data2 => HASH_TYPE_RADIO_DATA2,
                                                _ => unreachable!(),
                                            },
                                            |view: &mut RadioButton<ScriptHashType>| {
                                                view.select();
                                            },
                                        );
                                    }))
                                    .unwrap();
                            },
                            cb_sink_2.clone(),
                        ));
                    })
                    .align_center(),
                ),
        )
        .button("Search", move |siv| {
            let result = (|| {
                let lock_args = siv
                    .call_on_name(LOCK_ARGS, |view: &mut EditView| {
                        view.get_content().to_string()
                    })
                    .unwrap();
                let lock_hash = siv
                    .call_on_name(LOCK_HASH, |view: &mut EditView| {
                        view.get_content().to_string()
                    })
                    .unwrap();
                if lock_hash.len() < 2 {
                    bail!("Invalid lock hash");
                }
                let lock_hash = H256::from_str(&lock_hash[2..])
                    .with_context(|| anyhow!("Bad lock hash: {}", lock_hash))?;
                let lock_args = serde_json::from_value(json!(lock_args))
                    .with_context(|| anyhow!("Bad lock args: {}", lock_args))?;

                anyhow::Ok((lock_args, lock_hash))
            })();
            let script_hash_type = ScriptHashType::clone(&script_hash_type_radios.selection());
            let (lock_args, lock_hash) = match result {
                Ok((a, b)) => (a, b),
                Err(e) => {
                    siv.add_layer(
                        Dialog::around(TextView::new(format!("{:?}", e)))
                            .button("Close", |siv| {
                                siv.pop_layer();
                            })
                            .title("Error"),
                    );
                    return;
                }
            };
            let cb_sink = siv.cb_sink().clone();
            siv.add_layer(display_cells_dialog(
                &client_cloned,
                lock_args,
                lock_hash,
                script_hash_type,
                cb_sink,
            ));
        })
        .button("Close", |siv| {
            siv.pop_layer();
        })
}

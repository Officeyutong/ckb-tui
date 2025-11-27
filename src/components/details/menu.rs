use ckb_sdk::CkbRpcClient;
use cursive::{
    view::IntoBoxedView,
    views::{Button, Dialog, LinearLayout},
};

use crate::components::details::live_cells_searcher::live_cells_searcher;

pub fn details_menu(client: &CkbRpcClient) -> impl IntoBoxedView {
    let client_cloned = client.clone();
    Dialog::new()
        .content(
            LinearLayout::vertical().child(Button::new("Live Cells Searcher", move |siv| {
                siv.add_layer(live_cells_searcher(&client_cloned));
            })),
        )
        .title("Menu")
        .button("Close", |siv| {
            siv.pop_layer();
        })
}

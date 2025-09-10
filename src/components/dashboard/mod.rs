pub mod overview;

use std::sync::{Arc, atomic::AtomicBool};

use cursive::{
    view::{IntoBoxedView, Resizable}, views::{Button, DummyView, LinearLayout, Panel, TextView}, Cursive
};
use cursive_async_view::{AsyncState, AsyncView};

use crate::components::dashboard::overview::basic_info_dashboard;

pub fn dashboard(
    siv: &mut Cursive,
    loading_variable: Arc<AtomicBool>,
) -> impl IntoBoxedView + use<> {
    Panel::new(
        LinearLayout::vertical()
            .child(TextView::new("CKB Node Monitor").center())
            .child(
                LinearLayout::horizontal()
                    .child(Button::new("Overview", |_| ()).fixed_width(20))
                    .child(Button::new("Peers", |_| ()).fixed_width(20))
                    .child(Button::new("Network Impact", |_| ()).fixed_width(20))
                    .child(Button::new("System Info", |_| ()).fixed_width(20))
                    .child(Button::new("Tx Pool", |_| ()).fixed_width(20)),
            )
            .child(basic_info_dashboard())
            .child(Panel::new(TextView::new(
                "Press [Q] to quit, [Tab] to switch panels, [R] to refresh",
            )))
            .child(
                AsyncView::new(siv, move || {
                    if loading_variable.load(std::sync::atomic::Ordering::SeqCst) {
                        AsyncState::Pending
                    } else {
                        AsyncState::Available(DummyView::new())
                    }
                })
                .fixed_width(40),
            ),
    )
}

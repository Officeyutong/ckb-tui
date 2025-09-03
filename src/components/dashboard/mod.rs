pub mod overview;

use cursive::{
    view::{IntoBoxedView, Resizable},
    views::{Button, LinearLayout, Panel, TextView},
};

use crate::components::dashboard::overview::basic_info_dashboard;
pub fn dashboard() -> impl IntoBoxedView {
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
            .child(basic_info_dashboard()),
    )
}

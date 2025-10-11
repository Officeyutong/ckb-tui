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

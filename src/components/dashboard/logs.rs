use std::sync::{Arc, Mutex, mpsc};

use chrono::Local;
use cursive::{
    view::{IntoBoxedView, Nameable, Resizable, Scrollable},
    views::{LinearLayout, Panel, RadioGroup, TextView},
};
use cursive_table_view::{TableView, TableViewItem};
use rand::Rng;

use crate::{
    components::{
        DashboardState, UpdateToView,
        dashboard::{
            TUIEvent,
            logs::names::{
                LOGS_TABLE, SESSION_OVERVIEW_ERROR, SESSION_OVERVIEW_INFO, SESSION_OVERVIEW_WARN,
            },
        },
    },
    declare_names, update_text,
};

declare_names!(
    names,
    "logs_dashboard_",
    SESSION_OVERVIEW_INFO,
    SESSION_OVERVIEW_WARN,
    SESSION_OVERVIEW_ERROR,
    LOGS_TABLE
);
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogCategory {
    Info,
    Warn,
    Error,
}
#[derive(Clone)]
pub struct LogsItem {
    time: chrono::DateTime<Local>,
    category: LogCategory,
    source: String,
    message: String,
}
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum LogsColumn {
    Time,
    Category,
    Source,
    Message,
}
impl TableViewItem<LogsColumn> for LogsItem {
    fn to_column(&self, column: LogsColumn) -> String {
        match column {
            LogsColumn::Time => self.time.format("%Y-%m-%d %H:%M:%S%.3f %z").to_string(),
            LogsColumn::Category => match self.category {
                LogCategory::Info => String::from("Info"),
                LogCategory::Warn => String::from("Warn"),
                LogCategory::Error => String::from("Error"),
            },
            LogsColumn::Source => self.source.clone(),
            LogsColumn::Message => self.message.clone(),
        }
    }

    fn cmp(&self, other: &Self, column: LogsColumn) -> std::cmp::Ordering
    where
        Self: Sized,
    {
        match column {
            LogsColumn::Time => self.time.cmp(&other.time).reverse(),
            LogsColumn::Category => self.category.cmp(&other.category),
            LogsColumn::Source => self.source.cmp(&other.source),
            LogsColumn::Message => self.message.cmp(&other.message),
        }
    }
}
#[derive(Clone)]
pub enum FilterLogOption {
    All,
    Info,
    Warn,
    Error,
}
#[derive(Clone)]
pub struct LogsDashboardState {
    logs: Arc<Mutex<Vec<LogsItem>>>,
    filter_option: FilterLogOption,
}

impl LogsDashboardState {
    pub fn new() -> Self {
        Self {
            logs: Default::default(),
            filter_option: FilterLogOption::All,
        }
    }
}

impl DashboardState for LogsDashboardState {
    fn update_state(&mut self) -> anyhow::Result<()> {
        let mut guard = self.logs.lock().unwrap();
        let mut rng = rand::rng();

        guard.push(LogsItem {
            time: chrono::Local::now(),
            category: match rng.random_range(0..3) {
                0 => LogCategory::Info,
                1 => LogCategory::Warn,
                2 => LogCategory::Error,
                _ => unreachable!(),
            },
            source: "Test Source".to_string(),
            message: "Test Log".to_string(),
        });
        Ok(())
    }
    fn accept_event(&mut self, event: &TUIEvent) {
        match event {
            TUIEvent::FilterLogEvent(filter_log_option) => {
                self.filter_option = filter_log_option.clone()
            }
        }
    }
}
impl UpdateToView for LogsDashboardState {
    fn update_to_view(&self, siv: &mut cursive::Cursive) {
        let guard = self.logs.lock().unwrap();
        let (info, warn, error) =
            guard
                .iter()
                .fold((0, 0, 0), |(info, warn, error), item| match item.category {
                    LogCategory::Info => (info + 1, warn, error),
                    LogCategory::Warn => (info, warn + 1, error),
                    LogCategory::Error => (info, warn, error + 1),
                });
        update_text!(siv, SESSION_OVERVIEW_INFO, format!("🔵 INFO: {}", info));
        update_text!(siv, SESSION_OVERVIEW_WARN, format!("🟡 WARN: {}", warn));
        update_text!(siv, SESSION_OVERVIEW_ERROR, format!("🔴 ERROR: {}", error));
        siv.call_on_name(LOGS_TABLE, |view: &mut TableView<LogsItem, LogsColumn>| {
            let index = view.row();
            view.clear();
            for item in guard.iter() {
                if matches!(self.filter_option, FilterLogOption::Error)
                    && !matches!(item.category, LogCategory::Error)
                {
                    continue;
                }
                if matches!(self.filter_option, FilterLogOption::Info)
                    && !matches!(item.category, LogCategory::Info)
                {
                    continue;
                }
                if matches!(self.filter_option, FilterLogOption::Warn)
                    && !matches!(item.category, LogCategory::Warn)
                {
                    continue;
                }

                view.insert_item(item.clone());
            }
            if let Some(index) = index {
                view.set_selected_row(index);
            }
        });
    }
}
pub fn logs_dashboard(event_sender: mpsc::Sender<TUIEvent>) -> impl IntoBoxedView + use<> {
    let mut filter_group: RadioGroup<FilterLogOption> =
        RadioGroup::new().on_change(move |_, value: &FilterLogOption| {
            event_sender
                .send(TUIEvent::FilterLogEvent(value.clone()))
                .ok();
        });
    LinearLayout::vertical()
        .child(Panel::new(
            LinearLayout::vertical()
                .child(TextView::new("[Session OverView]"))
                .child(
                    LinearLayout::horizontal()
                        .child(
                            TextView::new(" ")
                                .with_name(SESSION_OVERVIEW_INFO)
                                .min_width(15),
                        )
                        .child(
                            TextView::new(" ")
                                .with_name(SESSION_OVERVIEW_WARN)
                                .min_width(15),
                        )
                        .child(
                            TextView::new(" ")
                                .with_name(SESSION_OVERVIEW_ERROR)
                                .min_width(15),
                        ),
                ),
        ))
        .child(Panel::new(
            LinearLayout::vertical()
                .child(TextView::new("[Stream]"))
                .child(
                    LinearLayout::horizontal()
                        .child(TextView::new("Filters:").min_width(10))
                        .child(
                            filter_group
                                .button(FilterLogOption::All, "All")
                                .min_width(10),
                        )
                        .child(
                            filter_group
                                .button(FilterLogOption::Info, "Info")
                                .min_width(10),
                        )
                        .child(
                            filter_group
                                .button(FilterLogOption::Warn, "Warn")
                                .min_width(10),
                        )
                        .child(
                            filter_group
                                .button(FilterLogOption::Error, "Error")
                                .min_width(10),
                        ),
                )
                .child(
                    TableView::<LogsItem, LogsColumn>::new()
                        .column(LogsColumn::Time, "Time", |c| c.width(30))
                        .column(LogsColumn::Category, "Category", |c| c.width(7))
                        .column(LogsColumn::Source, "Source", |c| c.width(15))
                        .column(LogsColumn::Message, "Message", |c| c.width(40))
                        .with_name(LOGS_TABLE)
                        .min_width(100)
                        .min_height(30)
                        .scrollable(),
                ),
        ))
}

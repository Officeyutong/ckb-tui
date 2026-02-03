use std::sync::{Arc, Mutex, mpsc};

use anyhow::{Context, anyhow};
use chrono::Local;
use cursive::{
    reexports::ahash::HashMap,
    view::{IntoBoxedView, Nameable, Resizable, Scrollable},
    views::{LinearLayout, Panel, RadioGroup, TextView},
};
use cursive_table_view::{TableView, TableViewItem};
use queue::Queue;
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;

use crate::{
    components::{
        DashboardState, UpdateToView,
        dashboard::{
            TUIEvent,
            logs::names::{
                LOGS_TABLE, SESSION_OVERVIEW_DEBUG, SESSION_OVERVIEW_ERROR, SESSION_OVERVIEW_INFO,
                SESSION_OVERVIEW_TRACE, SESSION_OVERVIEW_WARN,
            },
        },
    },
    declare_names, update_text,
    utils::create_subscription_client,
};

declare_names!(
    names,
    "logs_dashboard_",
    SESSION_OVERVIEW_INFO,
    SESSION_OVERVIEW_WARN,
    SESSION_OVERVIEW_ERROR,
    SESSION_OVERVIEW_DEBUG,
    SESSION_OVERVIEW_TRACE,
    LOGS_TABLE
);
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogCategory {
    Trace,
    Debug,
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
                LogCategory::Trace => String::from("Trace"),
                LogCategory::Debug => String::from("Debug"),
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
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}
#[derive(Clone)]
pub struct LogsDashboardInnerState {
    logs: Arc<Mutex<Queue<LogsItem>>>,
    category_sum: Arc<Mutex<HashMap<LogCategory, usize>>>,
    filter_option: FilterLogOption,
    stop_tx: tokio::sync::mpsc::Sender<()>,
}
#[derive(Clone)]
pub enum LogsDashboardState {
    WithTcpConn(LogsDashboardInnerState),
    WithoutTcpConn,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CkbLogEntry {
    /// The log message.
    pub message: String,
    /// The log level.
    pub level: LogCategory,
    /// The log target
    pub target: String,
    /// The date
    pub date: String,
}
fn update_log(data: &LogsDashboardInnerState, logs_entry: CkbLogEntry) {
    match data.category_sum.lock().unwrap().entry(logs_entry.level) {
        std::collections::hash_map::Entry::Occupied(mut occupied_entry) => {
            *occupied_entry.get_mut() += 1;
        }
        std::collections::hash_map::Entry::Vacant(vacant_entry) => {
            vacant_entry.insert(1);
        }
    };
}

impl LogsDashboardState {
    pub fn new(subscribe_addr: Option<String>) -> Self {
        if let Some(addr) = subscribe_addr {
            let (stop_tx, mut stop_rx) = tokio::sync::mpsc::channel(1);
            let result = Self::WithTcpConn(LogsDashboardInnerState {
                logs: Arc::new(Mutex::new(Queue::new())),
                category_sum: Default::default(),
                filter_option: FilterLogOption::All,
                stop_tx,
            });
            let self_cloned = result.clone();
            let tcp_addr = addr.to_string();
            std::thread::spawn(move || {
                log::info!("Logs subscription thread started");
                let result = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(o) => o,
                    Err(e) => {
                        log::error!("{:?}", e);
                        panic!("Unable to start tokio runtime");
                    }
                }
                .block_on(async move {
                    let mut logs_sub = create_subscription_client(&tcp_addr)
                        .await
                        .with_context(|| anyhow!("Unable to connect to: {}", tcp_addr))?
                        .subscribe::<CkbLogEntry>("logs")
                        .await
                        .with_context(|| anyhow!("Unable to subscribe logs"))?;

                    loop {
                        tokio::select! {
                            _ = stop_rx.recv() => {
                                break;
                            }
                            Some(Ok(r)) = logs_sub.next() => {
                                update_log(match self_cloned {
                                    LogsDashboardState::WithTcpConn(ref logs_dashboard_inner_state) => logs_dashboard_inner_state,
                                    LogsDashboardState::WithoutTcpConn =>unreachable!(),
                                }, r.1);
                            }
                        };
                    }
                    anyhow::Ok(())
                });
                log::info!("Tokio runtime exited: {:?}", result);
            });
            result
        } else {
            Self::WithoutTcpConn
        }
    }
}

impl DashboardState for LogsDashboardState {
    fn update_state(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
    fn accept_event(&mut self, event: &TUIEvent) {
        if let TUIEvent::FilterLogEvent(filter_log_option) = event {
            match self {
                LogsDashboardState::WithTcpConn(logs_dashboard_inner_state) => {
                    logs_dashboard_inner_state.filter_option = filter_log_option.clone()
                }
                LogsDashboardState::WithoutTcpConn => {}
            }
        }
    }
}

macro_rules! get_value {
    ($map:expr,$cat:expr) => {
        $map.get(&$cat).map(|x| *x).unwrap_or_default()
    };
}

impl UpdateToView for LogsDashboardState {
    fn update_to_view(&self, siv: &mut cursive::Cursive) {
        match self {
            LogsDashboardState::WithTcpConn(logs_dashboard_inner_state) => {
                let logs_guard = logs_dashboard_inner_state.logs.lock().unwrap();

                let count_guard = logs_dashboard_inner_state.category_sum.lock().unwrap();
                update_text!(
                    siv,
                    SESSION_OVERVIEW_TRACE,
                    format!("🔵 TRACE: {}", get_value!(count_guard, LogCategory::Trace))
                );
                update_text!(
                    siv,
                    SESSION_OVERVIEW_DEBUG,
                    format!("🔵 DEBUG: {}", get_value!(count_guard, LogCategory::Debug))
                );
                update_text!(
                    siv,
                    SESSION_OVERVIEW_INFO,
                    format!("🔵 INFO: {}", get_value!(count_guard, LogCategory::Info))
                );
                update_text!(
                    siv,
                    SESSION_OVERVIEW_WARN,
                    format!("🟡 WARN: {}", get_value!(count_guard, LogCategory::Warn))
                );
                update_text!(
                    siv,
                    SESSION_OVERVIEW_ERROR,
                    format!("🔴 ERROR: {}", get_value!(count_guard, LogCategory::Error))
                );
                siv.call_on_name(LOGS_TABLE, |view: &mut TableView<LogsItem, LogsColumn>| {
                    let index = view.row();
                    view.clear();
                    for item in logs_guard.vec().iter() {
                        if matches!(
                            logs_dashboard_inner_state.filter_option,
                            FilterLogOption::Error
                        ) && !matches!(item.category, LogCategory::Error)
                        {
                            continue;
                        }
                        if matches!(
                            logs_dashboard_inner_state.filter_option,
                            FilterLogOption::Info
                        ) && !matches!(item.category, LogCategory::Info)
                        {
                            continue;
                        }
                        if matches!(
                            logs_dashboard_inner_state.filter_option,
                            FilterLogOption::Warn
                        ) && !matches!(item.category, LogCategory::Warn)
                        {
                            continue;
                        }
                        if matches!(
                            logs_dashboard_inner_state.filter_option,
                            FilterLogOption::Debug
                        ) && !matches!(item.category, LogCategory::Debug)
                        {
                            continue;
                        }
                        if matches!(
                            logs_dashboard_inner_state.filter_option,
                            FilterLogOption::Trace
                        ) && !matches!(item.category, LogCategory::Trace)
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
            LogsDashboardState::WithoutTcpConn => {}
        }
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
                                .with_name(SESSION_OVERVIEW_TRACE)
                                .min_width(15),
                        )
                        .child(
                            TextView::new(" ")
                                .with_name(SESSION_OVERVIEW_DEBUG)
                                .min_width(15),
                        )
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
                                .button(FilterLogOption::Trace, "Trace")
                                .min_width(10),
                        )
                        .child(
                            filter_group
                                .button(FilterLogOption::Debug, "Debug")
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

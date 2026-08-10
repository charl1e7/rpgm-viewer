pub mod ui;

use egui::Color32;
use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum TimePrecision {
    Seconds,
    Milliseconds,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum TimeFormat {
    Utc,
    LocalTime,
    SinceStart,
    Hide,
}

pub(crate) struct LoggerStyle {
    pub enable_regex: bool,
    pub enable_ctx_menu: bool,
    pub enable_log_count: bool,
    pub enable_copy_button: bool,
    pub enable_search: bool,
    pub enable_max_log_output: bool,
    pub enable_levels_button: bool,
    pub enable_categories_button: bool,
    pub enable_time_button: bool,
    pub time_precision: TimePrecision,
    pub show_target: bool,
    pub time_format: TimeFormat,
    pub include_target: bool,
    pub include_level: bool,

    pub warn_color: Color32,
    pub error_color: Color32,
    pub highlight_color: Color32,
}

impl Default for LoggerStyle {
    fn default() -> Self {
        Self {
            show_target: true,
            enable_regex: true,
            enable_ctx_menu: true,
            include_target: true,
            include_level: true,
            time_format: TimeFormat::LocalTime,
            time_precision: TimePrecision::Seconds,
            warn_color: Color32::YELLOW,
            error_color: Color32::RED,
            highlight_color: Color32::LIGHT_GRAY,
            enable_log_count: true,
            enable_copy_button: true,
            enable_search: true,
            enable_max_log_output: true,
            enable_levels_button: true,
            enable_categories_button: true,
            enable_time_button: true,
        }
    }
}

pub struct LoggerUi {
    pub(crate) loglevels: [bool; log::Level::Trace as usize],
    pub(crate) search_term: String,
    pub(crate) regex: Option<Regex>,
    pub(crate) search_case_sensitive: bool,
    pub(crate) search_use_regex: bool,
    pub(crate) max_log_length: usize,
    pub(crate) style: LoggerStyle,
    pub(crate) search_cache: Vec<bool>,
    pub(crate) layout_cache: Vec<egui::text::LayoutJob>,
    pub(crate) cache_layouts: bool,
    pub(crate) autoscroll: bool,
}

impl Default for LoggerUi {
    fn default() -> Self {
        Self {
            loglevels: [true, true, true, false, false],
            search_term: String::new(),
            search_case_sensitive: false,
            regex: None,
            search_use_regex: false,
            max_log_length: 1000,
            style: LoggerStyle::default(),
            search_cache: Vec::new(),
            layout_cache: Vec::new(),
            cache_layouts: true,
            autoscroll: false,
        }
    }
}

pub fn logger_ui() -> LoggerUi {
    LoggerUi::default()
}

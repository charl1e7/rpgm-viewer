use std::sync::Mutex;

use egui::{Align, Color32, FontSelection, RichText, Style, text::LayoutJob};
use egui_logger::{LEVELS, LOGGER, Logger, Record};
use regex::RegexBuilder;

use super::{LoggerStyle, LoggerUi, TimeFormat, TimePrecision};

impl LoggerUi {
    #[inline]
    pub fn enable_regex(mut self, enable: bool) -> Self {
        self.style.enable_regex = enable;
        self
    }

    #[inline]
    pub fn enable_ctx_menu(mut self, enable: bool) -> Self {
        self.style.enable_ctx_menu = enable;
        self
    }

    #[inline]
    pub fn show_target(mut self, enable: bool) -> Self {
        self.style.show_target = enable;
        self
    }

    #[inline]
    pub fn include_target(mut self, enable: bool) -> Self {
        self.style.include_target = enable;
        self
    }

    #[inline]
    pub fn include_level(mut self, enable: bool) -> Self {
        self.style.include_level = enable;
        self
    }

    #[inline]
    pub fn enable_autoscroll(mut self, enable: bool) -> Self {
        self.autoscroll = enable;
        self
    }

    #[inline]
    pub fn enable_copy_button(mut self, enable: bool) -> Self {
        self.style.enable_copy_button = enable;
        self
    }

    #[inline]
    pub fn enable_log_count(mut self, enable: bool) -> Self {
        self.style.enable_log_count = enable;
        self
    }

    #[inline]
    pub fn enable_search(mut self, enable: bool) -> Self {
        self.style.enable_search = enable;
        self
    }

    #[inline]
    pub fn enable_max_log_output(mut self, enable: bool) -> Self {
        self.style.enable_max_log_output = enable;
        self
    }

    #[inline]
    pub fn enable_levels_button(mut self, enable: bool) -> Self {
        self.style.enable_levels_button = enable;
        self
    }

    #[inline]
    pub fn enable_categories_button(mut self, enable: bool) -> Self {
        self.style.enable_categories_button = enable;
        self
    }

    #[inline]
    pub fn enable_time_button(mut self, enable: bool) -> Self {
        self.style.enable_time_button = enable;
        self
    }

    #[inline]
    pub fn warn_color(mut self, color: Color32) -> Self {
        self.style.warn_color = color;
        self
    }

    #[inline]
    pub fn error_color(mut self, color: Color32) -> Self {
        self.style.error_color = color;
        self
    }

    #[inline]
    pub fn highlight_color(mut self, color: Color32) -> Self {
        self.style.highlight_color = color;
        self
    }

    #[inline]
    pub fn log_levels(mut self, log_levels: [bool; log::Level::Trace as usize]) -> Self {
        self.loglevels = log_levels;
        self
    }

    #[inline]
    pub fn enable_category(self, category: impl ToString, enable: bool) -> Self {
        LOGGER
            .lock()
            .as_mut()
            .expect("could not lock LOGGER")
            .categories
            .insert(category.to_string(), enable);
        self
    }

    #[inline]
    pub fn max_log_length(mut self, max_length: usize) -> Self {
        self.max_log_length = max_length;
        self
    }

    pub fn enable_cache_layouts(mut self, enable: bool) -> Self {
        self.cache_layouts = enable;
        self
    }

    pub(crate) fn log_ui(self) -> &'static Mutex<LoggerUi> {
        static LOGGER_UI: std::sync::OnceLock<Mutex<LoggerUi>> = std::sync::OnceLock::new();
        LOGGER_UI.get_or_init(|| self.into())
    }

    pub fn show(self, ui: &mut egui::Ui) {
        if let Ok(ref mut logger_ui) = self.log_ui().lock() {
            logger_ui.ui(ui);
        } else {
            ui.colored_label(Color32::RED, "Something went wrong loading the log");
        }
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui) {
        #[cfg(feature = "puffin")]
        puffin::profile_scope!("render logger UI");
        let Ok(ref mut logger) = LOGGER.lock() else {
            return;
        };

        let dropped_entries = logger.logs.len().saturating_sub(self.max_log_length);
        drop(logger.logs.drain(..dropped_entries));

        if dropped_entries > 0 {
            let drain_count = dropped_entries.min(self.search_cache.len());
            drop(self.search_cache.drain(..drain_count));
            if self.cache_layouts {
                let layout_drain = dropped_entries.min(self.layout_cache.len());
                drop(self.layout_cache.drain(..layout_drain));
            }
        }

        let mut search_changed = false;

        ui.horizontal(|ui| {
            if ui.button("Clear").clicked() {
                logger.logs.clear();
                self.search_cache.clear();
                self.layout_cache.clear();
            }

            if self.style.enable_levels_button {
                ui.menu_button("Log Levels", |ui| {
                    for level in LEVELS {
                        if ui
                            .selectable_label(self.loglevels[level as usize - 1], level.as_str())
                            .clicked()
                        {
                            self.loglevels[level as usize - 1] =
                                !self.loglevels[level as usize - 1];
                        }
                    }
                });
            }

            if self.style.enable_categories_button {
                ui.menu_button("Categories", |ui| {
                    if ui.button("Select All").clicked() {
                        for (_, enabled) in logger.categories.iter_mut() {
                            *enabled = true;
                        }
                    }

                    if ui.button("Unselect All").clicked() {
                        for (_, enabled) in logger.categories.iter_mut() {
                            *enabled = false;
                        }
                    }

                    for (category, enabled) in logger.categories.iter_mut() {
                        if ui.selectable_label(*enabled, category).clicked() {
                            *enabled = !*enabled;
                        }
                    }
                });
            }

            if self.style.enable_time_button {
                ui.menu_button("Time", |ui| {
                    search_changed |= ui
                        .radio_value(&mut self.style.time_format, TimeFormat::Utc, "UTC")
                        .changed();

                    search_changed |= ui
                        .radio_value(
                            &mut self.style.time_format,
                            TimeFormat::LocalTime,
                            "Local Time",
                        )
                        .changed();
                    search_changed |= ui
                        .radio_value(
                            &mut self.style.time_format,
                            TimeFormat::SinceStart,
                            "Since Start",
                        )
                        .changed();
                    search_changed |= ui
                        .radio_value(&mut self.style.time_format, TimeFormat::Hide, "Hide")
                        .changed();

                    ui.separator();

                    search_changed |= ui
                        .radio_value(
                            &mut self.style.time_precision,
                            TimePrecision::Seconds,
                            "Seconds",
                        )
                        .changed();
                    search_changed |= ui
                        .radio_value(
                            &mut self.style.time_precision,
                            TimePrecision::Milliseconds,
                            "Milliseconds",
                        )
                        .changed();
                });
            }
        });

        if self.style.enable_search {
            ui.horizontal(|ui| {
                ui.label("Search: ");
                let response = ui.text_edit_singleline(&mut self.search_term);

                if response.changed() {
                    search_changed = true;
                }

                if ui
                    .selectable_label(self.search_case_sensitive, "Aa")
                    .on_hover_text("Case sensitive")
                    .clicked()
                {
                    self.search_case_sensitive = !self.search_case_sensitive;
                    search_changed = true;
                }

                if self.style.enable_regex
                    && ui
                        .selectable_label(self.search_use_regex, ".*")
                        .on_hover_text("Use regex")
                        .clicked()
                {
                    self.search_use_regex = !self.search_use_regex;
                    search_changed = true;
                }

                if self.style.enable_regex && self.search_use_regex && search_changed {
                    self.regex = RegexBuilder::new(&self.search_term)
                        .case_insensitive(!self.search_case_sensitive)
                        .build()
                        .ok()
                }
            });
        }

        if self.style.enable_max_log_output {
            ui.horizontal(|ui| {
                ui.label("Max Log output");
                ui.add(egui::widgets::DragValue::new(&mut self.max_log_length).speed(1));
            });
        }

        ui.separator();

        let time_padding = logger.logs.last().map_or(0, |record| {
            format_time(record.time, &self.style, logger.start_time).len()
        });

        if self.cache_layouts {
            self.update_layout_cache(logger, time_padding, search_changed);
        }
        self.update_search_cache(logger, time_padding, search_changed);

        let filtered_logs: Vec<usize> = logger
            .logs
            .iter()
            .enumerate()
            .filter(|(_, r)| self.loglevels[r.level as usize - 1])
            .filter(|(_, record)| !matches!(logger.categories.get(&record.target), Some(false)))
            .filter(|(i, _)| self.search_cache.get(*i).copied().unwrap_or(true))
            .map(|(i, _)| i)
            .collect();

        let logs_displayed = filtered_logs.len();

        let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(ui.available_height() - 30.0)
            .stick_to_bottom(self.autoscroll)
            .show_rows(ui, row_height, logs_displayed, |ui, row_range| {
                for i in row_range {
                    let log_idx = filtered_logs[i];
                    let record = &logger.logs[log_idx];
                    let layout_job = if self.cache_layouts {
                        &self.layout_cache[log_idx]
                    } else {
                        &format_record(logger, &self.style, record, time_padding)
                    };

                    let response = ui.label(layout_job.clone());

                    if self.style.enable_ctx_menu {
                        response.clone().context_menu(|ui| {
                            if self.style.show_target {
                                ui.label(&record.target);
                            }
                            response.highlight();
                            let string_format = format!("[{}]: {}", record.level, record.message);

                            ui.vertical(|ui| {
                                ui.monospace(string_format);
                            });

                            if ui.button("Copy").clicked() {
                                ui.ctx().copy_text(layout_job.text.clone());
                            }
                        });
                    }
                }
            });

        ui.horizontal(|ui| {
            if self.style.enable_log_count {
                ui.label(format!("Log size: {}", logger.logs.len()));
                ui.label(format!("Displayed: {}", logs_displayed));
            }
            if self.style.enable_copy_button {
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Copy").clicked() {
                        let mut out_string = String::new();
                        logger
                            .logs
                            .iter()
                            .take(self.max_log_length)
                            .for_each(|record| {
                                out_string.push_str(
                                    &format_record(logger, &self.style, record, time_padding).text,
                                );
                                out_string.push_str(" \n");
                            });
                        ui.ctx().copy_text(out_string);
                    }
                });
            }
        });
    }

    fn match_string(&self, string: &str) -> bool {
        if self.search_use_regex {
            if let Some(matcher) = &self.regex {
                matcher.is_match(string)
            } else {
                false
            }
        } else if self.search_case_sensitive {
            string.contains(&self.search_term)
        } else {
            string
                .to_lowercase()
                .contains(&self.search_term.to_lowercase())
        }
    }

    fn update_layout_cache(&mut self, logger: &Logger, time_padding: usize, full_rebuild: bool) {
        let start = if full_rebuild {
            self.layout_cache.clear();
            0
        } else {
            self.layout_cache.len()
        };

        for record in logger.logs.iter().skip(start) {
            let job = format_record(logger, &self.style, record, time_padding);
            self.layout_cache.push(job);
        }
    }

    fn update_search_cache(&mut self, logger: &Logger, time_padding: usize, full_rebuild: bool) {
        let start = if full_rebuild {
            self.search_cache.clear();
            0
        } else {
            self.search_cache.len()
        };

        if self.search_term.is_empty() {
            self.search_cache
                .extend(std::iter::repeat_n(true, logger.logs.len() - start));
        } else {
            if self.cache_layouts {
                for layout in self.layout_cache.iter().skip(start) {
                    self.search_cache.push(self.match_string(&layout.text));
                }
            } else {
                for record in logger.logs.iter().skip(start) {
                    let job = format_record(logger, &self.style, record, time_padding);
                    self.search_cache.push(self.match_string(&job.text));
                }
            }
        }
    }
}

fn format_time(
    time: chrono::DateTime<chrono::Local>,
    style: &LoggerStyle,
    start_time: chrono::DateTime<chrono::Local>,
) -> String {
    let time = match (style.time_format, style.time_precision) {
        (TimeFormat::Utc, TimePrecision::Seconds) => time
            .to_utc()
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        (TimeFormat::Utc, TimePrecision::Milliseconds) => time
            .to_utc()
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        (TimeFormat::LocalTime, TimePrecision::Seconds) => time.format("%T").to_string(),
        (TimeFormat::LocalTime, TimePrecision::Milliseconds) => time.format("%T%.3f").to_string(),
        (TimeFormat::SinceStart, TimePrecision::Seconds) => {
            let duration = time - start_time;
            let h = duration.num_hours() % 24;
            let m = duration.num_minutes() % 60;
            let s = duration.num_seconds() % 60;
            match (h, m, s) {
                (0, 0, s) => format!("{s}s"),
                (0, m, s) => format!("{m}m {s}s"),
                (h, m, s) => format!("{h}h {m}m {s}s"),
            }
        }
        (TimeFormat::SinceStart, TimePrecision::Milliseconds) => {
            let duration = time - start_time;
            let h = duration.num_hours() % 24;
            let m = duration.num_minutes() % 60;
            let s = duration.num_seconds() % 60;
            let ms = duration.num_milliseconds() % 1000;
            match (h, m, s, ms) {
                (0, 0, 0, ms) => format!("{ms}ms"),
                (0, 0, s, ms) => format!("{s}s {ms}ms"),
                (0, m, s, ms) => format!("{m}m {s}s {ms}ms"),
                (h, m, s, ms) => format!("{h}h {m}m {s}s {ms}ms"),
            }
        }
        (TimeFormat::Hide, _) => String::new(),
    };
    if style.time_format == TimeFormat::Hide {
        time
    } else {
        time + " "
    }
}

fn format_record(
    logger: &Logger,
    logger_style: &LoggerStyle,
    record: &Record,
    time_padding: usize,
) -> LayoutJob {
    let level_str = if logger_style.include_level {
        format!("[{:5}] ", record.level)
    } else {
        String::new()
    };
    let target_str = if logger_style.include_target {
        format!(
            "{: <width$}: ",
            record.target,
            width = logger.max_category_length
        )
    } else {
        String::new()
    };
    let mut layout_job = LayoutJob::default();
    let style = Style::default();

    let mut date_str = RichText::new(format!(
        "{: >width$}",
        format_time(record.time, logger_style, logger.start_time),
        width = time_padding
    ))
    .monospace();
    match record.level {
        log::Level::Warn => date_str = date_str.color(logger_style.warn_color),
        log::Level::Error => date_str = date_str.color(logger_style.error_color),
        _ => {}
    }

    date_str.append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

    let highlight_color = match record.level {
        log::Level::Warn => logger_style.warn_color,
        log::Level::Error => logger_style.error_color,
        _ => logger_style.highlight_color,
    };

    RichText::new(level_str + &target_str)
        .monospace()
        .color(highlight_color)
        .append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

    let mut message = RichText::new(&record.message).monospace();
    match record.level {
        log::Level::Warn => message = message.color(logger_style.warn_color),
        log::Level::Error => message = message.color(logger_style.error_color),
        _ => {}
    }

    message.append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

    layout_job
}

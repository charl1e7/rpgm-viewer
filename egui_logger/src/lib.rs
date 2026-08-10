use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::Mutex;
use log::SetLoggerError;

pub const LEVELS: [log::Level; log::Level::Trace as usize] = [
    log::Level::Error,
    log::Level::Warn,
    log::Level::Info,
    log::Level::Debug,
    log::Level::Trace,
];

pub struct EguiLogger {
    max_level: log::LevelFilter,
    show_all_categories: bool,
    blacklisted: Vec<String>,
}

impl EguiLogger {
    fn new(
        max_level: log::LevelFilter,
        show_all_categories: bool,
        blacklisted: Vec<String>,
    ) -> Self {
        Self {
            max_level,
            show_all_categories,
            blacklisted,
        }
    }
}

pub struct Builder {
    max_level: log::LevelFilter,
    show_all_categories: bool,
    blacklisted: Vec<String>,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            max_level: log::LevelFilter::Debug,
            show_all_categories: true,
            blacklisted: vec![
                "tracing::span".to_string(),
                "tracing::span::active".to_string(),
            ],
        }
    }
}

impl Builder {
    pub fn build(self) -> EguiLogger {
        EguiLogger::new(self.max_level, self.show_all_categories, self.blacklisted)
    }

    pub fn max_level(mut self, max_level: log::LevelFilter) -> Self {
        self.max_level = max_level;
        self
    }

    pub fn show_all_categories(mut self, show_all_categories: bool) -> Self {
        self.show_all_categories = show_all_categories;
        self
    }

    pub fn default_blacklist(mut self, default_blacklist: bool) -> Self {
        if default_blacklist {
            self
        } else {
            self.blacklisted = vec![];
            self
        }
    }

    pub fn add_blacklist(mut self, target: impl ToString) -> Self {
        self.blacklisted.push(target.to_string());
        self
    }

    pub fn init(self) -> Result<(), SetLoggerError> {
        log::set_max_level(self.max_level);
        log::set_logger(Box::leak(Box::new(self.build())))
    }
}

impl log::Log for EguiLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.max_level
            && !self.blacklisted.contains(&metadata.target().to_string())
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata())
            && let Ok(ref mut logger) = LOGGER.lock()
        {
            logger.logs.push(Record {
                level: record.level(),
                message: record.args().to_string(),
                target: record.target().to_string(),
                time: chrono::Local::now(),
            });

            if !logger.categories.contains_key(record.target()) {
                logger
                    .categories
                    .insert(record.target().to_string(), self.show_all_categories);
                logger.max_category_length = logger.max_category_length.max(record.target().len());
            }
        }
    }

    fn flush(&self) {}
}

pub struct Record {
    pub level: log::Level,
    pub message: String,
    pub target: String,
    pub time: chrono::DateTime<chrono::Local>,
}

pub struct Logger {
    pub logs: Vec<Record>,
    pub categories: HashMap<String, bool>,
    pub max_category_length: usize,
    pub start_time: chrono::DateTime<chrono::Local>,
}

pub static LOGGER: LazyLock<Mutex<Logger>> = LazyLock::new(|| {
    Mutex::new(Logger {
        logs: Vec::new(),
        categories: HashMap::new(),
        max_category_length: 0,
        start_time: chrono::Local::now(),
    })
});

pub fn clear_logs() {
    LOGGER
        .lock()
        .expect("could not get access to logger")
        .logs
        .clear();
}

pub fn builder() -> Builder {
    Builder::default()
}
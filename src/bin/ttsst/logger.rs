use colored::*;
use log::*;
use ttsst::error::Result;

pub struct ConsoleLogger {
    default_level: Level,
}

impl log::Log for ConsoleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.default_level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            #[rustfmt::skip]
            let color = match record.level() {
                Level::Error => Color::Red,
                Level::Warn  => Color::Yellow,
                Level::Info  => Color::Green,
                Level::Debug => Color::Blue,
                Level::Trace => Color::Magenta,
            };

            let level_string = format!("{}:", record.level().to_string().to_lowercase())
                .color(color)
                .bold();

            #[rustfmt::skip]
            match record.level() {
                Level::Error => eprintln!("{} {}", level_string, record.args()),
                _            =>  println!("{} {}", level_string, record.args()),
            };
        }
    }

    fn flush(&self) {}
}

impl ConsoleLogger {
    #[must_use = "You must call init() to begin logging"]
    pub fn new() -> Self {
        ConsoleLogger {
            default_level: Level::Trace,
        }
    }

    #[must_use = "You must call init() to begin logging"]
    pub fn init(self, log_level: LevelFilter) -> Result<()> {
        log::set_boxed_logger(Box::new(self))?;
        log::set_max_level(log_level);
        Ok(())
    }
}

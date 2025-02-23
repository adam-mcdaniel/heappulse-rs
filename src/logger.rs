use tracing::{Event, Subscriber, Level};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;
use core::fmt::{Debug, Write};
use tracing_subscriber::prelude::*;
use libc::{write, STDOUT_FILENO};

// No-allocation logger that writes directly to stdout
struct NoAllocLogger;

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for NoAllocLogger {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut writer = LowLevelWriter;

        // Write the colorized log level
        let color = match *event.metadata().level() {
            Level::TRACE => "\x1b[36m", // Blue
            Level::DEBUG => "\x1b[34m", // Green
            Level::INFO => "\x1b[32m", // Cyan
            Level::WARN => "\x1b[33m", // Yellow
            Level::ERROR => "\x1b[31m", // Red
        };
        let reset = "\x1b[0m";

        let _ = write!(writer, "[{color}{}{reset}] ", event.metadata().level());

        event.record(&mut |_field: &tracing_core::Field, value: &dyn Debug| {
            let _ = write!(writer, "{:?}", value);
        });

        let _ = writeln!(writer, ""); // Newline after log
    }
}


// Direct low-level `stdout` writer using `libc::write`
struct LowLevelWriter;

impl Write for LowLevelWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        unsafe {
            write(STDOUT_FILENO, s.as_ptr() as *const _, s.len());
        }
        Ok(())
    }
}

static mut LOGGING_INITIALIZED: bool = false;

pub unsafe fn init_logging() {
    unsafe {
        if LOGGING_INITIALIZED {
            return;
        }
        LOGGING_INITIALIZED = true;
    }
    // // Set up logging with the `tracing` crate, with debug level logging.
    // let _ = tracing_subscriber::fmt::SubscriberBuilder::default()
    //     .with_max_level(tracing::Level::DEBUG)
    //     .without_time()
    //     .init();
    // tracing::register

    // let subscriber = tracing_subscriber::Registry::default().with(NoAllocLogger);
    let subscriber = tracing_subscriber::Registry::default().with(NoAllocLogger.with_filter(crate::config::LOG_LEVEL));
    // let subscriber = tracing_subscriber::Registry::default().with(NoAllocLogger.with_filter(tracing_subscriber::filter::LevelFilter::OFF));
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting tracing default failed");
}

#[cfg(test)]
mod tests {
    use super::init_logging;

    #[test]
    fn test() {
        unsafe {
            init_logging();
        }
    
        // Test log messages
        tracing::trace!("This is a trace log.");
        tracing::debug!("This is a debug log.");
        tracing::info!("Logging variable example {x}.", x=42);
        tracing::warn!("Warning message.");
        tracing::error!("Error message.");
    }
}
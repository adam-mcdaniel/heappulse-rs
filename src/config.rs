use crate::interval::IntervalConfig;
use tracing_subscriber::filter::LevelFilter;

pub const ALIGN_ALLOCATIONS_TO_PAGE_SIZE: bool = true;

pub const UNPROTECT_READ_WRITE_ON_FAULT: bool = false;

pub const INTERVAL_CONFIG: IntervalConfig = IntervalConfig {
    interval_ms: 1000,
};

pub const MAX_TRACKED_ALLOCATIONS: usize = 65536;

pub const LOG_LEVEL: LevelFilter = LevelFilter::TRACE;
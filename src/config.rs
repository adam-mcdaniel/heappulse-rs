use crate::interval::IntervalTestConfig;

pub const ALIGN_ALLOCATIONS_TO_PAGE_SIZE: bool = true;

pub const UNPROTECT_READ_WRITE_ON_FAULT: bool = false;

pub const INTERVAL_CONFIG: IntervalTestConfig = IntervalTestConfig {
    interval_ms: 1000,
};

pub const MAX_TRACKED_ALLOCATIONS: usize = 1024;

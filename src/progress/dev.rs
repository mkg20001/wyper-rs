use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize};

/// track status of device progress
#[derive(Default)]
pub struct DevProgress {
    pub current_pass: AtomicUsize,
    pub current_bytes_written: AtomicU64,
    pub bytes_total: AtomicU64,
    pub current_bad_sectors: AtomicU64,
    pub pass_result: Mutex<Vec<PassResult>>,
}

pub struct PassResult {
    pub bytes_written: u64,
    pub bad_sectors: u64,
    pub failure: Option<anyhow::Error>,
}

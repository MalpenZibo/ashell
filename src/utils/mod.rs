pub mod launcher;
pub mod remote_value;
pub use crate::IndicatorState;

/// Wall-clock milliseconds (mod 100000) for debug open-latency probes.
pub fn debug_wall_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() % 100_000)
        .unwrap_or(0)
}

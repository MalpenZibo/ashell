use std::time::Duration;

pub(crate) const RETRY_DELAY: Duration = Duration::from_secs(5);

/// Warns about the first of a run of failures, and the retries after it only at
/// debug level. A macro, so the log names the service that failed.
#[cfg(any(feature = "brightness", feature = "rfkill"))]
macro_rules! failed {
    ($failing:expr, $($message:tt)+) => {
        if std::mem::replace($failing, true) {
            log::debug!($($message)+)
        } else {
            log::warn!($($message)+)
        }
    };
}

#[cfg(any(feature = "brightness", feature = "rfkill"))]
pub(crate) use failed;

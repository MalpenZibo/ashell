//! How a length of time is written: a timer's countdown and a stopwatch's
//! elapsed time.

use std::time::Duration;

/// A countdown: `MM:SS`, or `H:MM:SS` past an hour.
pub fn clock(left: Duration) -> String {
    let seconds = left.as_secs();
    let (hours, minutes, seconds) = (seconds / 3600, seconds % 3600 / 60, seconds % 60);

    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

/// A stopwatch: `MM:SS.CC`, or `H:MM:SS.CC` past an hour, to the
/// hundredth.
pub fn stopwatch(elapsed: Duration) -> String {
    let seconds = elapsed.as_secs();
    let (hours, minutes, seconds) = (seconds / 3600, seconds % 3600 / 60, seconds % 60);
    let centis = elapsed.subsec_millis() / 10;

    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}.{centis:02}")
    } else {
        format!("{minutes:02}:{seconds:02}.{centis:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_countdown_is_minutes_and_seconds_until_an_hour() {
        assert_eq!(clock(Duration::from_secs(0)), "00:00");
        assert_eq!(clock(Duration::from_secs(59)), "00:59");
        assert_eq!(clock(Duration::from_secs(60)), "01:00");
        assert_eq!(clock(Duration::from_secs(3599)), "59:59");
        assert_eq!(clock(Duration::from_secs(3600)), "1:00:00");
        assert_eq!(clock(Duration::from_secs(3661)), "1:01:01");
    }

    #[test]
    fn a_stopwatch_counts_hundredths() {
        assert_eq!(stopwatch(Duration::ZERO), "00:00.00");
        assert_eq!(stopwatch(Duration::from_millis(9)), "00:00.00");
        assert_eq!(stopwatch(Duration::from_millis(10)), "00:00.01");
        assert_eq!(stopwatch(Duration::from_millis(999)), "00:00.99");
        assert_eq!(stopwatch(Duration::from_millis(1000)), "00:01.00");
        assert_eq!(stopwatch(Duration::from_millis(61_234)), "01:01.23");
        assert_eq!(stopwatch(Duration::from_millis(3_661_005)), "1:01:01.00");
    }
}

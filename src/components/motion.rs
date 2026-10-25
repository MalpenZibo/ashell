//! Shared animation timings and easings.

use iced::animation::Easing;
use std::time::Duration;

/// Bar layout shifts: module sizes, workspace indicators, center alignment.
pub const FAST: Duration = Duration::from_millis(140);

/// Containers opening or closing: menus, sub-menus, toasts, the OSD.
pub const BASE: Duration = Duration::from_millis(180);

/// For elements entering the screen.
pub const EASE_OUT: Easing = Easing::EaseOutCubic;

/// For elements leaving the screen.
pub const EASE_IN: Easing = Easing::EaseInCubic;

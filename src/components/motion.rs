//! Shared animation timings and easings.

use iced::animation::Easing;
use std::time::Duration;

/// Hover and press feedback.
pub const INSTANT: Duration = Duration::from_millis(90);

/// Bar layout shifts: module sizes, workspace indicators, center alignment.
pub const FAST: Duration = Duration::from_millis(140);

/// Containers opening or closing: menus, sub-menus, toasts, the OSD.
pub const BASE: Duration = Duration::from_millis(180);

/// For elements entering the screen.
pub const EASE_OUT: Easing = Easing::EaseOutCubic;

/// For elements leaving the screen.
pub const EASE_IN: Easing = Easing::EaseInCubic;

/// For movement that starts and ends at rest, such as a menu moving between bar
/// buttons.
pub const EASE_IN_OUT: Easing = Easing::EaseInOutCubic;

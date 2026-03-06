# Tray Service Cleanup Opportunities

This document outlines potential cleanup and improvement opportunities in the tray service implementation.

## High Priority

### Issue 1: Incorrect timestamp usage in menu_item_selected ✅ **COMPLETED**

**Location:** `src/services/tray/mod.rs:775`

**Current Code:**

```rust
chrono::offset::Local::now().timestamp_subsec_micros()
```

**Issue:** Using `timestamp_subsec_micros()` only returns the microseconds portion (0-999,999), not the full timestamp. According to the DBusMenu specification, the timestamp should be an X11 timestamp (milliseconds since X server started). Since this is a Wayland-native application without X11 access, using 0 is the correct approach to indicate no timestamp is available.

**Suggested Fix:**

```rust
// Use 0 to indicate no X11 timestamp available (common in Wayland)
const timestamp: u32 = 0;
```

**Status:** Fixed - Changed to use timestamp=0 and renamed method to `menu_item_selected`

---

### Issue 2: Duplicate fallback icon lookup logic ✅ **COMPLETED**

**Location:** `src/services/tray/mod.rs:453-475`

**Current Code:**

```rust
.or_else(|| {
    // Try each fallback candidate
    let mut found_icon = None;
    for candidate in fallbacks.iter() {
        if let Some(icon) = get_icon_from_name(candidate) {
            found_icon = Some(icon);
            break;
        }
    }
    found_icon
})
```

This logic is duplicated in both the `Ok(icons)` and `Err(_)` branches.

**Suggested Fix:**

```rust
fn try_fallback_icons(fallbacks: &[String]) -> Option<TrayIcon> {
    fallbacks.iter().find_map(|name| get_icon_from_name(name))
}
```

**Status:** Fixed - Extracted duplicate logic into `try_fallback_icons` helper function

---

## Medium Priority

### Issue 3: Duplicate ARGB to RGBA conversion logic ✅ **COMPLETED**

**Location:** `src/services/tray/mod.rs:442-446` and `627-631`

**Current Code:**

```rust
// Convert ARGB to RGBA: [A, R, G, B] -> [R, G, B, A]
// rotate_left(1) moves the alpha byte from position 0 to position 3
for pixel in i.bytes.chunks_exact_mut(4) {
    pixel.rotate_left(1);
}
```

**Suggested Fix:**

```rust
fn convert_argb_to_rgba(icon: Icon) -> Icon {
    let mut icon = icon;
    for pixel in icon.bytes.chunks_exact_mut(4) {
        pixel.rotate_left(1);
    }
    icon
}
```

**Status:** Fixed - Extracted duplicate logic into `convert_argb_to_rgba` helper function in dbus.rs

---

### Issue 4: Duplicate fallback building logic ✅ **COMPLETED**

**Location:** `src/services/tray/mod.rs:411-429`

**Current Code:**

```rust
let mut fallbacks = Vec::new();
if let Some(ref icon_name) = icon_name_prop {
    let icon_name = icon_name.trim();
    if !icon_name.is_empty() && !fallbacks.contains(&icon_name.to_string()) {
        fallbacks.push(icon_name.to_string());
    }
}
if let Some(ref id) = id_prop {
    let id = id.trim();
    if !id.is_empty() && !fallbacks.contains(&id.to_string()) {
        fallbacks.push(id.to_string());
    }
}
if let Some(ref title) = title_prop {
    let title = title.trim();
    if !title.is_empty() && !fallbacks.contains(&title.to_string()) {
        fallbacks.push(title.to_string());
    }
}
```

**Suggested Fix:**

```rust
fn add_fallback(fallbacks: &mut Vec<String>, value: Option<&String>) {
    if let Some(value) = value {
        let value = value.trim();
        if !value.is_empty() && !fallbacks.contains(&value.to_string()) {
            fallbacks.push(value.to_string());
        }
    }
}

// Usage:
let mut fallbacks = Vec::new();
add_fallback(&mut fallbacks, icon_name_prop.as_ref());
add_fallback(&mut fallbacks, id_prop.as_ref());
add_fallback(&mut fallbacks, title_prop.as_ref());
```

**Status:** Fixed - Extracted duplicate logic into `add_fallback` helper function

---

### Issue 5: Duplicate discovery logic in discover_items ✅ **COMPLETED**

**Location:** `src/services/tray/dbus.rs:119-166`

**Current Code:**
The two branches (for services containing "StatusNotifierItem" vs. introspection) do nearly identical work:

- Get the name owner
- Create a signal emitter
- Register the item

**Suggested Fix:** Extract into a helper function:

```rust
async fn try_register_item(
    conn: &Connection,
    interface: &zbus::object_server::InterfaceRef<StatusNotifierWatcher>,
    dbus_proxy: &DBusProxy<'_>,
    name: &BusName<'_>,
    service_path: &str,
) {
    let sender = match dbus_proxy.get_name_owner(name.clone()).await {
        Ok(owner) => owner,
        Err(_) => return,
    };

    let mut watcher = interface.get_mut().await;
    let emitter = match SignalEmitter::new(conn, OBJECT_PATH) {
        Ok(emitter) => emitter,
        Err(err) => {
            warn!("Failed to create signal emitter for registration: {err}");
            return;
        }
    };
    watcher
        .register_status_notifier_item_manual(
            service_path,
            sender.into_inner(),
            &emitter,
        )
        .await;
}
```

**Status:** Fixed - Extracted duplicate logic into `try_register_item` helper function

---

## Low Priority

### Issue 6: Hardcoded timeout value

**Location:** `src/services/tray/dbus.rs:209`

**Current Code:**

```rust
tokio::time::timeout(tokio::time::Duration::from_secs(5), proxy.introspect()).await
```

**Suggested Fix:** Make this a named constant with documentation:

```rust
const INTROSPECTION_TIMEOUT_SECS: u64 = 5;
// Usage:
tokio::time::timeout(tokio::time::Duration::from_secs(INTROSPECTION_TIMEOUT_SECS), proxy.introspect()).await
```

---

### Issue 7: Simplify clone logic in duplicate handling ✅ **COMPLETED**

**Location:** `src/services/tray/dbus.rs:251-261`

**Current Code:**

```rust
if let Some((old_sender, old_service)) = self.items.iter().find(|(_, s)| *s == &service) {
    // Clone the key before the async call to avoid borrow conflicts
    let old_sender = old_sender.clone();
    // Emit unregistered signal for the old entry before removing it
    if let Err(err) = Self::status_notifier_item_unregistered(emitter, old_service).await {
        warn!(
            "Failed to emit status_notifier_item_unregistered for duplicate service '{old_service}': {err}"
        );
    }
    self.items.remove(&old_sender);
}
```

**Suggested Fix:** Clone before the if let to simplify:

```rust
if let Some((old_sender, old_service)) = self.items.iter().find(|(_, s)| *s == &service).map(|(k, v)| (k.clone(), v.clone())) {
    // Emit unregistered signal for the old entry before removing it
    if let Err(err) = Self::status_notifier_item_unregistered(emitter, &old_service).await {
        warn!(
            "Failed to emit status_notifier_item_unregistered for duplicate service '{old_service}': {err}"
        );
    }
    self.items.remove(&old_sender);
}
```

**Status:** Fixed - Simplified clone logic by using `.map()` to clone before the if let

---

### Issue 8: Poor variable naming ✅ **COMPLETED**

**Location:** `src/services/tray/mod.rs:854`

**Current Code:**

```rust
let name_cb = name.clone();
```

**Issue:** The variable name `name_cb` suggests a callback, but it's actually just a cloned string.

**Suggested Fix:** Rename to `name_clone` or similar:

```rust
let name_clone = name.clone();
```

**Status:** Fixed - Renamed `name_cb` to `name_clone` for clarity

---

### Issue 9: Duplicate directory collection logic ✅ **COMPLETED**

**Location:** `src/services/tray/mod.rs:128-153` and `333-365`

**Current Code:**
`desktop_directories()` and `icon_directories()` have nearly identical structure, only differing in the subdirectory path they append.

**Suggested Fix:** Refactor to share logic:

```rust
fn collect_xdg_directories(subdirs: &[&str]) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(data_home) = env::var("XDG_DATA_HOME") {
        let base = PathBuf::from(data_home);
        for subdir in subdirs {
            dirs.push(base.join(subdir));
        }
    }

    if let Ok(home) = env::var("HOME") {
        let base = PathBuf::from(home);
        for subdir in subdirs {
            dirs.push(base.join(".local/share").join(subdir));
        }
    }

    let data_dirs = env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    for dir in data_dirs.split(':') {
        if dir.is_empty() {
            continue;
        }
        let base = PathBuf::from(dir);
        for subdir in subdirs {
            dirs.push(base.join(subdir));
        }
    }

    for subdir in subdirs {
        dirs.push(PathBuf::from("/usr/share").join(subdir));
    }

    dirs.sort();
    dirs.dedup();
    dirs
}

fn desktop_directories() -> Vec<PathBuf> {
    collect_xdg_directories(&["applications"])
}

fn icon_directories() -> Vec<PathBuf> {
    collect_xdg_directories(&["icons", "pixmaps"])
}
```

**Status:** Fixed - Extracted duplicate logic into `collect_xdg_directories` helper function

---

## Future Improvements

### Performance Issue: Stream Lifecycle in TrayService

**Location:** `src/services/tray/mod.rs:694-700` (start_listening function)

**Current Behavior:**
When a new tray item registers, the event loop breaks and `TrayService::events()` is called again, which rebuilds all D-Bus proxies and streams for every tray item, even those that haven't changed.

**Impact:**

- CPU spikes when new apps with tray icons launch
- Unnecessary D-Bus traffic from reconnecting to all existing services
- Increased memory churn from recreating all streams
- Problem scales poorly with the number of tray items

**Suggested Solution:**
Use `StreamMap` from `tokio-stream` to dynamically manage streams:

```rust
// Store StreamMap in State::Active
enum State {
    Init,
    Active(zbus::Connection, StreamMap<usize, BoxStream<TrayEvent>>),
    Error,
}

// When a new item registers, add its streams to the StreamMap
// When an item unregisters, remove its streams from the StreamMap
// Never break the event loop - just update the StreamMap dynamically
```

**Benefits:**

- Allows dynamic insertion/removal of streams
- No need to tear down existing streams
- Existing tray items continue uninterrupted
- Only the new item's stream is added

**Implementation Complexity:**
This is a significant architectural change that requires:

1. Adding tokio-stream dependency (already present)
2. Refactoring State to hold StreamMap
3. Refactoring event loop to use StreamMap instead of select_all()
4. Managing lifecycle of individual streams
5. Handling stream errors without breaking the entire event loop

**Recommendation:**
This should be implemented as a future performance optimization, but is not a blocker for current work. The current implementation works correctly, just inefficiently.

---

## Summary

### Priority Breakdown

- **High Priority:** Issues that may cause bugs or significant code duplication
- **Medium Priority:** Issues that improve code maintainability and reduce duplication
- **Low Priority:** Minor improvements and code quality enhancements

### Recommended Action Order

1. Fix Issue 1 (incorrect timestamp) - potential bug
2. Address Issue 2 (duplicate fallback lookup) - significant duplication
3. Consider Issues 3-5 (other duplications) - improve maintainability
4. Address Issues 6-9 (minor improvements) - as time permits

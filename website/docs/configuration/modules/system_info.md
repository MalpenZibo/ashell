---
sidebar_position: 8
---

# System Info

This module provides information about the system such as:

- CPU usage
- Memory usage
- Disk space
- Network IP
- Network speed
- Temperature

It changes the indicator color based on the related value.  
For example, if the CPU usage is above 80%, the indicator will be red.

By default, the module will display the CPU usage, memory usage, and temperature.

## Indicators

Using the `indicators` configuration, you can select which indicators
to display in the status bar.

These are the available indicators:

### CPU

The CPU indicator displays the current CPU usage as a percentage by default.

To enable this indicator, add `Cpu` to the `indicators` configuration.

You can change the display format using the `format` option in `[system_info.cpu]`:

- `"Percentage"` (default) — shows CPU usage as a percentage (e.g., `72%`)
- `"Frequency"` — shows the average CPU frequency in GHz (e.g., `3.42 GHz`)

### Memory

The Memory indicator displays the current memory usage as a percentage by default.

To enable this indicator, add `Memory` to the `indicators` configuration.

You can change the display format using the `format` option in `[system_info.memory]`:

- `"Percentage"` (default) — shows memory usage as a percentage (e.g., `64%`)
- `"Fraction"` — shows used and total memory in GiB (e.g., `5.12/15.89 GiB`)

### Memory Swap

The Memory Swap indicator displays the current memory swap usage as a percentage by default.

To enable this indicator, add `MemorySwap` to the `indicators` configuration.

:::info
Memory Swap uses the same `format` setting as Memory (`[system_info.memory]`). If you set Memory to `"Fraction"`, the swap indicator will also display as a fraction.
:::

### Disk

The Disk indicator displays the disk space usage for a specific path.

To enable this indicator, add `{ Disk = "path" }` or `{ Disk = "path", Name = "label" }` to the `indicators` configuration,
where `path` is the **mount point** of the filesystem you want to monitor (e.g. `/`
or `/home`, as shown by `df`) and `label` is an optional name to display for it.

:::warning
`Disk` matches the mount point, not the block device. A device node such as
`/dev/sda1` never matches, and the indicator is silently omitted from the bar.
:::

You can change the display format using the `format` option in `[system_info.disk]`:

- `"Percentage"` (default) — shows disk usage as a percentage (e.g., `54%`)
- `"Fraction"` — shows used and total disk space in GB (e.g., `256.00/512.00 GB`)

#### Filtering Disk Mounts

By default, all non-removable disks with available space are shown in the system info menu. You can use the `mounts` option in `[system_info.disk]` to specify which mount points to display.

If `mounts` is not specified, all disks are shown (default behavior).

```toml
[system_info.disk]
mounts = ["/", "/home"]
warn_threshold = 80
alert_threshold = 90
format = "Percentage"
```

With the above configuration, only the disks mounted at `/` and `/home` will be displayed in the menu.

#### Example

To monitor the home directory disk space, you can add the following to your configuration:

```toml
[system_info]
indicators = [ { Disk = "/home" } ]
```

Or if you want to display the directory disk space with an optional name, for example `bob` instead of its full path:

```toml
[system_info]
indicators = [ { Disk = "/my/long/path/to/mount/called/bob", Name = "bob" } ]
```

### IpAddress

The IpAddress indicator displays the current IP address of the system.

To enable this indicator, add `IpAddress` to the `indicators` configuration.

### DownloadSpeed

The DownloadSpeed indicator displays the current download speed  
of the system's network connection.

To enable this indicator, add `DownloadSpeed` to the `indicators` configuration.

### UploadSpeed

The UploadSpeed indicator displays the current upload speed  
of the system's network connection.

To enable this indicator, add `UploadSpeed` to the `indicators` configuration.

### Temperature

The Temperature indicator displays the current temperature from the configured sensor.

To enable this indicator, add `Temperature` to the `indicators` configuration.

The sensor is configured with a single `sensor` option that accepts either a sensor **type** keyword (auto-detected) or an **exact sensor label** (manual override).

Sensor type keywords (auto-detected):

- `"Cpu"` (default) — CPU temperature (coretemp/k10temp)
- `"Gpu"` — GPU temperature (amdgpu/nouveau)
- `"Acpi"` — ACPI thermal zone (acpitz)
- `"Nvme"` — NVMe SSD temperature

Any other string is treated as an exact sensor label, overriding auto-detection (e.g. `sensor = "k10temp Tctl"`). If the configured label isn't found on your system, ashell falls back to `Cpu` auto-detection.

```toml
[system_info.temperature]
sensor = "Cpu"              # auto-detect by type
# sensor = "k10temp Tctl"   # or pin an exact sensor label
```

The temperature **unit** follows your locale / unit system (the global `region` option) by default. Set `units` to `"Celsius"` or `"Fahrenheit"` to override it for this indicator only — useful if you want Fahrenheit for the weather but Celsius for hardware temperatures.

```toml
[system_info.temperature]
units = "Celsius"           # override the locale unit system
```

ashell reads hardware sensors through the `sysinfo` crate, which builds each
label as `<hwmon chip name> <sensor label>`, so the value you write in `sensor`
is **not** the bare label printed by `sensors`. `sensors` shows the chip as a
heading (`k10temp-pci-00c3`) and the sensor below it (`Tctl`); the label ashell
expects joins the two: `k10temp Tctl`.

To list the labels exactly as ashell sees them, read them straight from `hwmon`:

```bash
for d in /sys/class/hwmon/hwmon*; do
  chip=$(cat "$d/name")
  for f in "$d"/temp*_label; do
    [ -e "$f" ] && echo "$chip $(cat "$f")"
  done
done
```

Alternatively, run ashell with `level = "info"` in the `[logging]` section: it
logs the label of the sensor it auto-detected, and warns when a configured label
was not found.

For NVMe SSDs, you'll need to find the model number first:

```bash
# Get NVMe model number
lsblk -d -o name,model
# Output example:
# NAME    MODEL
# nvme0n1 CT1000T705SSD3
```

Common sensor labels include:

- `acpitz temp1` - ACPI thermal zone
- `coretemp Package id 0` - Intel CPU temperature
- `k10temp Tctl` - AMD Ryzen CPU temperature
- `amdgpu edge` - AMD GPU temperature
- `nvme Composite MODEL_NAME` - NVMe SSD temperature (use model from `lsblk` output)

## Polling Interval

You can configure how often the system information is refreshed using the `interval` option (in seconds). The default is `5` seconds.

```toml
[system_info]
indicators = [ "Cpu", "Memory", "Temperature" ]
interval = 10
```

Higher values reduce CPU usage at the cost of less frequent updates.

## Display Formats

The `Cpu`, `Memory` and `Disk` indicator types each support a `format` option
that controls how their value is displayed in the status bar and menu. The
format is configured in the corresponding `[system_info.<type>]` section.

`MemorySwap` has no section of its own: it reuses `[system_info.memory]` for
both its `format` and its thresholds.

The remaining indicators (`Temperature`, `IpAddress`, `DownloadSpeed`,
`UploadSpeed`) always show their value directly and have no `format` option.

:::info
Warning and alert color thresholds remain active regardless of the display format. For temperature, thresholds are interpreted in the displayed unit (`units`, or the one determined by your locale / unit system), so if the indicator shows Fahrenheit, set your thresholds in Fahrenheit. The built-in temperature defaults are 60 / 80 in Celsius and 140 / 176 in Fahrenheit; ashell picks the pair matching the resolved unit.
:::

#### Example

```toml
[system_info.cpu]
format = "Frequency"

[system_info.memory]
format = "Fraction"

[system_info.disk]
format = "Fraction"
```

## Warning and Alert Thresholds

You can also configure the warning and alert thresholds for the following indicators:

- CPU
- Memory (RAM and Swap use the same thresholds)
- Disk
- Temperature

Each one is configured in its own table, `[system_info.<indicator>]`, where
`<indicator>` is one of `cpu`, `memory`, `disk` or `temperature`:

```toml
[system_info.cpu]
warn_threshold = 60
alert_threshold = 80
```

:::info
`warn_threshold` must be lower than `alert_threshold`. If it is not, ashell logs
a warning and lowers `warn_threshold` to match `alert_threshold`.

`MemorySwap` has no table of its own and reuses the `[system_info.memory]`
thresholds.
:::

## Default Configuration

```toml
[system_info]
indicators = [ "Cpu", "Memory", "Temperature" ]
interval = 5

[system_info.cpu]
warn_threshold = 60
alert_threshold = 80
format = "Percentage"

[system_info.memory]
warn_threshold = 70
alert_threshold = 85
format = "Percentage"

[system_info.disk]
warn_threshold = 80
alert_threshold = 90
format = "Percentage"
# mounts = ["/", "/home"]  # uncomment to whitelist specific mount points

[system_info.temperature]
sensor = "Cpu"  # type keyword ("Cpu", "Gpu", "Acpi", "Nvme") or an exact sensor label
# sensor = "k10temp Tctl"  # example: pin an exact sensor label
# warn_threshold and alert_threshold are unset by default. Omit them and ashell
# uses 60 / 80 in Celsius, or 140 / 176 when the resolved unit is Fahrenheit.
# units is unset by default: omit it to follow the locale unit system.
```

:::warning
TOML has no `None` literal. Leave `warn_threshold`, `alert_threshold` and
`units` out of the file to keep their defaults. Writing `warn_threshold = None`
is a parse error that makes ashell fall back to the *entire* default config.
:::

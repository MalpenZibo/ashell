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
where `path` is the path to the disk you want to monitor and `label` is an optional name to display for the disk.

You can change the display format using the `format` option in `[system_info.disk]`:

- `"Percentage"` (default) — shows disk usage as a percentage (e.g., `54%`)
- `"Fraction"` — shows used and total disk space in GB (e.g., `256.00/512.00 GB`)

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

By default, the temperature sensor used is `acpitz temp1` (ACPI thermal zone).
You can configure which sensor to use with the `sensor` option in the `[system_info.temperature]` section.

You can also change the display format using the `format` option:

- `"Celsius"` (default) — shows temperature in Celsius (e.g., `52°C`)
- `"Fahrenheit"` — shows temperature in Fahrenheit (e.g., `125°F`)

To see available sensors on your system, you can check the output of `sensors` command or
look at the component labels returned by the sysinfo library.

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

Each indicator type supports a `format` option that controls how its value is displayed in the status bar and menu. The format is configured in the corresponding `[system_info.<type>]` section.

:::info
Warning and alert color thresholds remain active regardless of the display format. For temperature, thresholds are interpreted in the configured unit — so if you use `"Fahrenheit"`, set your thresholds in Fahrenheit (e.g., `warn_threshold = 140`).
:::

#### Example

```toml
[system_info.cpu]
format = "Frequency"

[system_info.memory]
format = "Fraction"

[system_info.temperature]
format = "Fahrenheit"

[system_info.disk]
format = "Fraction"
```

## Warning and Alert Thresholds

You can also configure the warning and alert thresholds for the following indicators:

- CPU
- Memory (RAM and Swap use the same thresholds)
- Disk
- Temperature

To configure a threshold, you can add the following to your configuration:

```toml
[system_info.threshold_type]
warn_threshold = 60
alert_threshold = 80
```

Where **threshold_type** is the type of indicator you want to  
configure and can be one of:

- `cpu`
- `memory`
- `disk`
- `temperature`

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

[system_info.temperature]
warn_threshold = 60
alert_threshold = 80
sensor = "acpitz temp1"
format = "Celsius"
```

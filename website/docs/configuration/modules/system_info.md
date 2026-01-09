---
sidebar_position: 6
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

The CPU indicator displays the current CPU usage as a percentage.

To enable this indicator, add `Cpu` to the `indicators` configuration.

### Memory

The Memory indicator displays the current memory usage as a percentage.

To enable this indicator, add `Memory` to the `indicators` configuration.

### Memory Swap

The Memory Swap indicator displays the current memory swap usage as a percentage.

To enable this indicator, add `MemorySwap` to the `indicators` configuration.

### Disk

The Disk indicator displays the disk space usage for a specific path.

To enable this indicator, add `{ Disk = "path" }` or `{ Disk = "path", Name = "label" }` to the `indicators` configuration,
where `path` is the path to the disk you want to monitor and `label` is an optional name to display for the disk.

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

[system_info.cpu]
warn_threshold = 60
alert_threshold = 80

[system_info.memory]
warn_threshold = 70
alert_threshold = 85

[system_info.disk]
warn_threshold = 80
alert_threshold = 90

[system_info.temperature]
warn_threshold = 60
alert_threshold = 80
sensor = "acpitz temp1"
```

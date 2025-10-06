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

To enable this indicator, add `{ Disk = "path" }` to the `indicators` configuration,
where `path` is the path to the disk you want to monitor.

#### Example

To monitor the home directory disk space, you can add the following to your configuration:

```toml
[system_info]
indicators = [ { Disk = "/home" } ]
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

The Temperature indicator displays the current temperature of the system's CPU.

To enable this indicator, add `Temperature` to the `indicators` configuration.

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
```

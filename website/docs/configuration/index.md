# ⚙️ Configuration

All configuration options are stored in the `config.toml` file, located at:

```bash
~/.config/ashell
```

:::info

Ashell does **not** create this file automatically.

:::

Ashell watches this file for changes and will apply updates
immediately—so you can tweak the configuration while Ashell is running.

See more about the [TOML format](https://toml.io/en/).

## Command-line parameters

You can pass a configuration file to Ashell using the `--config-path` parameter:

```bash
ashell --config-path /path/to/config.toml
```

This allows you to use a different configuration file instead of the default one.

Ashell will still watch this file for changes and apply updates immediately.

## IPC messaging

Ashell exposes a Unix socket for controlling a running instance. The same binary
acts as a client when invoked with the `msg` subcommand:

```bash
ashell msg <command>
```

Available commands:

| Command                  | Description                          |
| ------------------------ | ------------------------------------ |
| `toggle-visibility`      | Toggle the bar on/off                |
| `volume-up`              | Increase sink volume by 5%           |
| `volume-down`            | Decrease sink volume by 5%           |
| `volume-toggle-mute`     | Toggle sink mute                     |
| `microphone-up`          | Increase source volume by 5%         |
| `microphone-down`        | Decrease source volume by 5%         |
| `microphone-toggle-mute` | Toggle source mute                   |
| `brightness-up`          | Increase screen brightness by 5%     |
| `brightness-down`        | Decrease screen brightness by 5%     |
| `airplane-toggle`        | Toggle airplane mode                 |

Volume, microphone, brightness, and airplane commands show an OSD (On-Screen Display)
overlay by default. Add `--no-osd` to suppress it:

```bash
ashell msg volume-up --no-osd
```

The socket is created at `$XDG_RUNTIME_DIR/ashell.sock`.

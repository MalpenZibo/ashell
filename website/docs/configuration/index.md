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

## How commands are executed

Every configuration option that takes a command string is run through
`bash -c "<your command>"`. Shell features are therefore available:
pipes, `&&`, `$(...)` substitution, globs, redirection and environment
variables all work as they would in an interactive shell.

The options executed this way are:

| Where | Options |
| ----- | ------- |
| [Updates](./modules/updates.md) | `check_cmd`, `update_cmd` |
| [Settings](./modules/settings.md), power menu | `lock_cmd`, `suspend_cmd`, `hibernate_cmd`, `reboot_cmd`, `shutdown_cmd`, `logout_cmd` |
| [Settings](./modules/settings.md), "more" buttons | `audio_sinks_more_cmd`, `audio_sources_more_cmd`, `wifi_more_cmd`, `vpn_more_cmd`, `bluetooth_more_cmd` |
| [Settings](./modules/settings.md), custom buttons | `command`, `status_command` |
| [Custom modules](./modules/custom_module.md) | `command`, `listen_cmd`, `on_right_click`, `on_middle_click`, `on_scroll_up`, `on_scroll_down` |

Because a shell interprets these strings, remember to quote paths and
arguments that contain spaces, and to escape `$` where you want a literal
dollar sign rather than a variable.

:::warning

Anything in these options runs with your user's full privileges, as soon as
the relevant module loads or the relevant button is pressed. Note that
`check_cmd` and `listen_cmd` run automatically, without any interaction.

Treat `config.toml` with the same care as `~/.bashrc`: don't paste command
options from sources you don't trust, and don't make the file writable by
other users.

:::

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
| `toggle-airplane-mode`   | Toggle airplane mode                 |
| `toggle-idle-inhibitor`  | Toggle idle inhibitor                |


Volume, microphone, brightness, airplane and idle inhibitor commands show an OSD (On-Screen Display)
overlay by default. Add `--no-osd` to suppress it:

```bash
ashell msg volume-up --no-osd
```

The socket is created at `$XDG_RUNTIME_DIR/ashell.sock`.

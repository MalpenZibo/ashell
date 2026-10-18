# ⚙️ Configuration

All configuration options are stored in the `config.toml` file, located at:

```bash
~/.config/ashell
```

:::info

Ashell does **not** create this file automatically.

:::

Ashell watches this file for changes and will apply updates
immediately, so you can tweak the configuration while Ashell is running.

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
| `toggle-visibility`      | Toggle the bar on/off                                        |
| `volume-up`              | Increase sink volume by `settings.volume_step` (default 5%)  |
| `volume-down`            | Decrease sink volume by `settings.volume_step` (default 5%)  |
| `volume-toggle-mute`     | Toggle sink mute                                             |
| `microphone-up`          | Increase source volume by 5% (fixed)                         |
| `microphone-down`        | Decrease source volume by 5% (fixed)                         |
| `microphone-toggle-mute` | Toggle source mute                                           |
| `brightness-up`          | Increase screen brightness by 5% of the device maximum       |
| `brightness-down`        | Decrease screen brightness by 5% of the device maximum       |
| `toggle-airplane-mode`   | Toggle airplane mode                                         |
| `toggle-idle-inhibitor`  | Toggle idle inhibitor                                        |
| `load-config <file>`            | Load another config without restarting ashell         |
| `notifications-clear [--id <id>]` | Close all notifications, or a single notification |
| `notifications-invoke [--id <id>] [action]` | Invoke an action and close its notification |
| `notifications-list` | Print notifications as a JSON array, newest first |

Only `volume-up` / `volume-down` honour
[`settings.volume_step`](./modules/settings.md#volume-step); the microphone and
brightness steps are fixed. Sink volume is also capped by
[`settings.max_volume`](./modules/settings.md#max-volume).


Volume, microphone, brightness, airplane and idle inhibitor commands show an OSD (On-Screen Display)
overlay by default. Add `--no-osd` to suppress it:

```bash
ashell msg volume-up --no-osd
```

The socket is created at `$XDG_RUNTIME_DIR/ashell.sock`. When
`$XDG_RUNTIME_DIR` is unset or unusable, ashell falls back to
`$TMPDIR/ashell-<uid>/ashell.sock` (typically `/tmp/ashell-1000/ashell.sock`).
That directory must be owned by you with mode `0700`; otherwise ashell runs
without IPC and `ashell msg` refuses to connect.

If another ashell instance already owns the socket, the new instance still
starts but runs without IPC and logs a warning.

### Direct socket access

The socket accepts one newline-delimited JSON request per connection and returns
one JSON response. Command names match the CLI; argument fields use snake_case:

```json
{"command":"toggle-visibility"}
{"command":"volume-up","no_osd":true}
{"command":"load-config","file":"/home/user/.config/ashell/another config.toml"}
```

`no_osd` defaults to `false`. Config paths sent directly to the socket must be
absolute and point to a valid config. Requests, including the newline, may be at
most 4096 bytes.

Responses are `{"status":"ok"}` or
`{"status":"error","message":"description of the error"}`. The CLI continues
to print `ok` for successful actions and exits with status 1 on errors. Replies
come from the app after it handles the request; notification clear/invoke replies
wait for the operation to finish and the local notification list to update.

The server handles up to 32 connections at once. Sending a request and writing a
response each have a 5-second deadline; waiting for the app to handle a request
has a 30-second deadline. A response timeout does not cancel a command already
submitted to the app, so it may still complete. The CLI also limits socket reads
and writes to avoid waiting indefinitely.

`{"command":"notifications-list"}` returns
`{"status":"ok","notifications":[...]}`. The CLI prints only the array. List
responses may exceed the request size limit. See [Notifications](./modules/notifications.md#ipc-commands)
for the record format and command arguments.

The socket protocol uses JSON only. Scripts sending the previous plain-text
format must migrate; existing `ashell msg` commands and flags are unchanged.

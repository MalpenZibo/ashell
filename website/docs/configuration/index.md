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

## Command line parameters

You can pass a configuration file to Ashell using the `--config-path` parameter:

```bash
ashell --config-path /path/to/config.toml
```

This allows you to use a different configuration file than the default one.

Ashell will still watch this file for changes and apply updates immediately.

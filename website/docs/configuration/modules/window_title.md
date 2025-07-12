---
sidebar_position: 5
---

# Window Title

Displays the title of the currently focused window.

Using the `mode` field, you can configure what information to show:

- `Title`: the window title, which is the default
- `Class`: the window class

You can also configure the maximum title length, after which the title will be  
truncated, using the `truncate_title_after_length` field.

The default value is 150 characters.

## Example

```toml
[window_title]
mode = "Title"
truncate_title_after_length = 75
```

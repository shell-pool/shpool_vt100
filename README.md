# shpool_vt100

A fork of the vt100 crate with some tweaks for implementing
shpool. See the main vt100 crate for more details.

# Maintanance Mode

This crate is in maintainance mode and we're actively working on switching to shpool-vterm,
which you can try out by putting

```
session_restore_engine = "vterm"
```

in your `~/.config/shpool/config.toml`

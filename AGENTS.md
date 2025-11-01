## Project overview

This project is a jupyter client for Neovim, written in Rust and callable
in Neovim via Lua using the `mlua` crate in module mode.

## Testing

Integration tests are files like `lua_tests/test_*.lua`. You can run these
using `luajit`.

When you run you should probably ensure that all log messages are printed by
setting the environmental variable`RUST_LOG=trace`. Log messages are written to
`jet.log`.

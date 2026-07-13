// On macOS, this cdylib is loaded by Neovim, which provides the Lua
// symbols at runtime. Tell the linker not to resolve them at link time.
// (Same effect as `-C link-arg=-undefined -C link-arg=dynamic_lookup`
// in .cargo/config.toml, but scoped to this crate and picked up
// regardless of the working directory cargo is invoked from.)
//
// If we ever ship musl Linux targets, they'll need
// `-C target-feature=-crt-static` (dynamic C runtime, so Neovim's libc
// wins). That's a rustc flag, not a link arg, so it belongs in
// .cargo/config.toml under `[target.*-linux-musl]`, not here.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-cdylib-link-arg=-undefined");
        println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
    }
}

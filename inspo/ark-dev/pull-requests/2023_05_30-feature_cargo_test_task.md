# Add `cargo test` task

> <https://github.com/posit-dev/ark/pull/17>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/rstudio/positron/issues/578

This registers `cargo test` as the default for the `Tasks: Run Test Task` command (if you don't set it as the default, there is also a Node one that can appear). This comes straight from the built in cargo test task that you can run with `CMD+P` -> `Tasks: Run Task` -> `rust: cargo test`.

---

Unfortunately, unlike the `Tasks: Run Build Task` command, `Tasks: Run Test Task` isn't hooked up to a keybinding by default. But you can easily add it to one with:

- `CMD + K` chorded to `CMD + S` to open the keybinding menu
- Search for `Tasks: Run Test Task`, it should already exist but not have a keybinding
- Double click on the cell under the `Keybinding` header and bind it to `CMD+SHIFT+T`

It told me there was already a keybinding for `CMD+SHIFT+T` related to `View: Reopen Closed Editor` but I don't care about that one so I just ignored it. The user preference seems to take precedence so it seems ok to have both in there.

---

We can't have project wide keybindings https://github.com/Microsoft/vscode/issues/4504

We could make an extension that _only_ has keybindings in it, but that seems overkill right now


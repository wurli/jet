# Eliminate ad hoc creation of function completion items

> <https://github.com/posit-dev/ark/issues/849>
> 
> * Author: @jennybc
> * State: OPEN
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABSPDwDw", name = "enhancement", description = "New feature or request", color = "C5DEF5"), list(id = "LA_kwDOJkuGPc8AAAABwXx7Aw", name = "area: language server", description = "", color = "C2E0C6")

Loose end discovered while working on #819:

https://github.com/posit-dev/ark/blob/bcdaed2b9f89b04df0b912778fde0cf059f40b62/crates/ark/src/lsp/completions/completion_item.rs#L122-L135

`completion_item_from_assignment()`, which is used for document completions, hand crafts a function completion item, which means it doesn't benefit from all the smarts in `completion_item_from_function()`.

I actually found this problem when writing tests for #819, because my first crack at it was with a user-defined function in the global environment. I wondered why things weren't working! This issue is a reminder to me to come back and route this through `completion_item_from_function()` and to look hard for any other instances of similar.


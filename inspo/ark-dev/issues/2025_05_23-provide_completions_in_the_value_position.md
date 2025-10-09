# Provide completions in the "value" position

> <https://github.com/posit-dev/ark/issues/812>
> 
> * Author: @jennybc
> * State: OPEN
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABwXx7Aw", name = "area: language server", description = "", color = "C2E0C6")

This is a variation / follow-up on #770.

We generally don't provide completions in the "value" position, in the sense of `fn(name = value)`. This feels weird to me. It's admittedly a somewhat pedantic point, but I don't think it's *pure* pedantry. It feels like this violates the logic of how completions should work.

*Pro tip: An explicit gesture to get completions is Ctrl/Cmd + Space. This works in some contexts where, say, pressing "tab" inserts a literal `\t` vs. triggering completions.*

Consider this code:

```r
a_kinda_long_name <- letters[1:3]
another_long_name <- letters[4:6]

append(x = a_kinda_long_name, y = another_long_name)
```

Below I indicate cursor position with `@`.

If you explicitly ask for completions at `x = @`, you get 'No suggestions'.

<img width="335" alt="Image" src="https://github.com/user-attachments/assets/ab329a8a-a3dc-4831-97ae-f1df6e407717" />

But as soon as you type anything, e.g. `x = a@`, you get dozens of completions, including `a_kinda_long_name` and `another_long_name`.

<img width="542" alt="Image" src="https://github.com/user-attachments/assets/da3ae523-6948-4463-bd8d-77631244fce5" />

It feels like the implicit contract is that the completions for `x = an@` are a subset of the completions for `x = a@` are a subset of the completions for `x =@`. And we're currently violating that.

You can actually see that in this recording, where I request completions before I've typed anything, then get stuck in the land of 'No suggestions' until I backspace all the way back to the `"="`:

https://github.com/user-attachments/assets/bc71ed5e-7447-4c12-9317-bf5b1a1a91e8

The place to intervene is around here, but we don't yet have an easy way to add something that is morally `is_value_position(node)` to this condition:

https://github.com/posit-dev/ark/blob/2013ca15704284d36ea0908a3b497832dfdff49f/crates/ark/src/lsp/completions/sources/composite.rs#L77-L78

(Side note: I also noticie some duplication in the above completion lists 🤔. I have some ideas about that. But it's a separate matter, in any case.)


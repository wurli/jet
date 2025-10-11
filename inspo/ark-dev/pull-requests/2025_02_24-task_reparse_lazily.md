# Generate srcrefs lazily

> <https://github.com/posit-dev/ark/pull/719>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

This PR disables the generation of srcrefs on session init to avoid the associated memory cost. Addresses https://github.com/posit-dev/positron/issues/5050. Instead srcref generation is lazily triggered when the user calls `debug()` or `debugonce()`.

- Refactor hook mechanism a little

- Hook `debug()` and `debugonce()` to trigger srcref regeneration

- Prevent idle tasks from running in debug prompts. We should have done that before too. It's unsafe to be mutating functions in package and namespace envs when R code is running. Srcref generation should only be done when it's empty.

    This required refactoring the `select!` in `read_console()` around `Select`. The macro is convenient but doesn't allow for dynamism which is required to only poll idle tasks in non-debug prompts.

I left `browser()` alone because wrapping this primitive in a closure creates weirdness in the call stack that confuses debugging. The proper way to deal with it is to have the LSP detect `browser()` calls, add a breakpoint, and we'd treat the breakpoint creation as a hint that we need to regenerate srcrefs.

With this change I get Ark to cold boot at 125mb

![Screenshot 2025-02-24 at 10 50 41](https://github.com/user-attachments/assets/1c46a99b-544f-4251-bfd0-14ca33f95a91)

Compared to the 150mb of RStudio

![Screenshot 2025-02-24 at 10 51 05](https://github.com/user-attachments/assets/3fafa5d6-f88e-4fc0-8ad5-aefa20370cf2)



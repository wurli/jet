# Improve diagnostics of `<<-`, `->`, and `->>`

> <https://github.com/posit-dev/ark/pull/309>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/2704

Since apparently some people out there use right assignment 😆 

- Left super assignment wasn't adding the symbol to `document_symbols`, so future usage of the symbol would still be marked even though immediate usage in the `x <<- 1` call itself wasn't marked.
- Right assignment wasn't supported at all, but is now
- Right super assignment wasn't supported at all, but is now

Added some tests for these cases too, yay tests!

<img width="218" alt="Screenshot 2024-04-11 at 12 56 54 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/f51e5ddd-ff1e-4413-9b6c-362b9994269f">



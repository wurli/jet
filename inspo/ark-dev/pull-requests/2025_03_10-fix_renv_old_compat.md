# Expand patch renv support to renv 1.0.1 and earlier

> <https://github.com/posit-dev/ark/pull/736>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/6681

This expands on https://github.com/posit-dev/ark/pull/383 where we automatically return `"n"` when we detect we are in a `readline()` in the renv autoloader. The code goes through a different code path in 1.0.1 so we need to do something different there.

- In renv 1.0.9 and up, we should never end up here
- In renv 1.0.2 to 1.0.8, we return `"n"` to escape the `renv:::ask()` call
- In renv 1.0.1 and earlier, we return `"Leave project library empty"` to escape the specific `renv:::menu()` call

I can't figure out any better way to do this due to the way renv bootstraps itself. Luckily this should be very static "patch" support since CRAN renv no longer needs anything like this.

Here is the rladies repo at least booting up now, you get no indication that you need to `restore()` but I think this is the best we can do right now.

<img width="878" alt="Screenshot 2025-03-10 at 3 54 54 PM" src="https://github.com/user-attachments/assets/2854c945-e1ad-43ae-969c-675eeb58ec36" />




# Fix release mode warning

> <https://github.com/posit-dev/ark/pull/685>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

This is only actually needed when `#[cfg(debug_assertions)]` is on, so it was warning in release mode


# Remove unnecessary mut from OnceCell and OnceLock

> <https://github.com/posit-dev/ark/pull/428>
> 
> * Author: @yutannihilation
> * State: MERGED
> * Labels: 

Fix https://github.com/posit-dev/positron/issues/3912

`OnceCell::set()` doesn't require mutability.

https://doc.rust-lang.org/std/cell/struct.OnceCell.html#method.set

## @lionel- at 2024-07-08T07:45:54Z

Thanks!
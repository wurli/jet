# More ergonomic `Vec<bool>` to `RObject` conversion

> <https://github.com/posit-dev/ark/pull/345>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Using `&Vec<bool>` and `From` instead of `TryFrom` (at least until we add an allocator that can fail, then we could switch back to `TryFrom` if it makes sense)


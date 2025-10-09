# Maybe unsound in RLocal::new

> <https://github.com/posit-dev/ark/issues/653>
> 
> * Author: @lwz23
> * State: OPEN
> * Labels: 

Hello, thank you for your contribution in this project, I am scanning the unsoundness problem in rust project.
I notice the following code:
```
pub struct RLocal<T: Copy> {
    old_value: T,
    variable: *mut T,
}

impl<T> RLocal<T>
where
    T: Copy,
{
    pub fn new(new_value: T, variable: *mut T) -> RLocal<T> {
        unsafe {
            let old_value = libr::get(variable);
            libr::set(variable, new_value);

            Self {
                old_value,
                variable,
            }
        }
    }
}
```
Considering that `pub mod raii`, and `new`  is also a pub function. I assume that users can directly call this function. This potential situation could result in `libr::get` being called to  a null pointer, and might trigger undefined behavior (UB). For safety reasons, I felt it necessary to report this issue. If you have performed checks elsewhere that ensure this is safe, please don’t take offense at my raising this issue.
I suggest Several possible fixes: 
1. If there is no external usage for `RLocal` or `new`, they should not marked as `pub`, at least its `new` should not marked as `pub` 
2. `new` method should add additional check for null pointer.
3. mark new method as unsafe and proper doc to let users know that they should provide valid Pointers.

## @lionel- at 2024-12-12T09:16:26Z

It's really an internal function from an internal crate (neither are meant to be used in other projects). We can make it `pub(crate)` for clarity.

## @lwz23 at 2024-12-23T10:02:45Z

Thanks for your reply, I think maybe same problem for https://github.com/posit-dev/ark/blob/a569d6ee15d85723aec5625620c0593a7a8476a7/crates/harp/src/vector/list.rs#L65, the `index` ptr offset is not been varify.
and 
https://github.com/posit-dev/ark/blob/a569d6ee15d85723aec5625620c0593a7a8476a7/crates/ark/src/sys/unix/console.rs#L13
https://github.com/posit-dev/ark/blob/a569d6ee15d85723aec5625620c0593a7a8476a7/crates/ark/src/sys/windows/console.rs#L37
https://github.com/posit-dev/ark/blob/1366044e69062bc88a8d1bcc6f474c980f6f8166/crates/harp/src/table.rs#L162
I understand that these functions are not intended for external use, but in that case it might be more appropriate to declare them as `pub(crate)`. This will ensure that future projects on these modules will not cause possible unsound problems, thank you!

## @lionel- at 2025-01-06T09:58:32Z

Note that we have cross-crate internal dependencies too.
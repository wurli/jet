# Update time and ahash

> <https://github.com/posit-dev/ark/pull/426>
> 
> * Author: @yutannihilation
> * State: MERGED
> * Labels: 

Fix https://github.com/posit-dev/positron/issues/3911.

Update the time crate and ahash crate to solve compilation errors. Honestly I don't understand the details, but it seems the Rust compiler will be more strict about type ambiguity.

## @lionel- at 2024-07-08T07:35:20Z

Thanks!
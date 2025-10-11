# Add `List` and `ListIter` types

> <https://github.com/posit-dev/ark/pull/481>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

Branched from posit-dev/positron#480.

~These are bare bones for now. Implemented outside of the `Vector` trait as I was struggling to make the types work, it seemed simpler to implement the interface manually for lists.~

Edit: Now implemented with the `Vector` trait.

- Create lists with `List::create([...])`. Currently no support for named elements.
- Retrieve iterator over `SEXP` elements with `iter()`. Or create one from an unwrapped list with `ListIter::new()`.

New supporting infra:

- `harp::Error::OutOfMemory` error, progress towards posit-dev/ark#693.
- `harp::alloc_list()`. Checks for OOM errors.
- `harp::as_error()`, handy to convert any error to `harp::error::AnyhowError`, e.g. with `?`
- `harp::as_r_ssize()` to safely convert from `usize` to `r_xlen_t`.
- `harp::list_cbegin()`
- `harp::is_identical()`


# Correct string handling in hashmap conversion; fix test

> <https://github.com/posit-dev/ark/pull/99>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This change fixes some issues in string handling that lead to intermittent unit test failures (only on Linux), such as https://github.com/posit-dev/amalthea/actions/runs/6264634957/job/17011748218.

```
---- object::tests::test_tryfrom_RObject_hashmap_string stdout ----
thread 'object::tests::test_tryfrom_RObject_hashmap_string' panicked at 'called `Result::unwrap()` on an `Err` value:
InvalidUtf8(Utf8Error { valid_up_to: 10, error_len: Some(1) })', crates/harp/src/object.rs:743:64
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

Unfortunately, the intermittent nature of the failures meant that the PR that introduced them, https://github.com/posit-dev/amalthea/pull/97, passed CI. 

The fixes are as follows:
- Use `Rf_mkCharLenCE` rather than `Rf_mkChar`; the latter appears to frequently generate strings that are not terminated.
- Use `Rf_translateCharUTF8` rather than `R_CHAR`. This wasn't in code introduced in #97, but appears to be an issue with the existing code (which did not formerly have unit test coverage). 
- While I was in the code, I switched the protection mechanism for the values vector to `RProtect` since that seems to be the idiom used elsewhere in this file.

I've set up a Linux environment to validate these changes and was able to both consistently reproduce the failure there and pass consistently after these changes.

## @kevinushey at 2023-09-25T21:16:13Z

> the latter appears to frequently generate strings that are not terminated.

Presumably this is because Rust strings are not actually null terminated? Hence it's important to use the R APIs that accept a length, or to construct a CString or CStr that is itself explicitly null-terminated.

## @jmcphers at 2023-09-26T17:26:45Z

> I think we'd favour going through CString instead of using length-parameterised ctors? 

FWIW I tried doing this and I don't think it's preferable as it winds up being more verbose and less efficient. Creating a `CString` from an `&String` adds an allocation and also requires additional error handling. 
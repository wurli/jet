# Soundness Issue: Unsafe raw pointer operation exposed through safe public API r_show_message

> <https://github.com/posit-dev/ark/issues/729>
> 
> * Author: @lwz23
> * State: CLOSED
> * Labels: 

Hello,

Thank you for your work on this project. While auditing Rust code for memory safety issues, I discovered a potential soundness problem in the `interface` module.

## Issue Description

The function `r_show_message` is marked as `pub` and accepts a raw pointer (`*const c_char`) without being marked as `unsafe`, but internally performs unsafe operations without validating the pointer:

```
impl RMain {
    ...................
    fn show_message(&self, buf: *const c_char) {
        let message = unsafe { CStr::from_ptr(buf) };
        let message = message.to_str().unwrap().to_string();
    ....................
    }
}

pub extern "C-unwind" fn r_show_message(buf: *const c_char) {
    let main = RMain::get();
    main.show_message(buf);
}
```
According to Rust's safety guarantees, any function that could cause undefined behavior when called with valid parameters must be marked as unsafe. In this case, a null or invalid pointer would lead to undefined behavior, which violates Rust's safety principles.

##POC
```
use ark::interface::r_show_message;
use std::ptr;

fn main() {
    let invalid_ptr = ptr::null();
    r_show_message(invalid_ptr); // This should require unsafe, but doesn't
}
```


##Result
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.35s
     Running `target/debug/poc1`
Segmentation fault (core dumped)
```

##Suggested Fixes
1. Preferred: Mark the function as unsafe:
```
pub unsafe extern "C-unwind" fn r_show_message(buf: *const c_char) {
    // ...
}
```
2. Alternative: Reduce visibility if not intended for external use:
```
pub(crate) extern "C-unwind" fn r_show_message(buf: *const c_char) {
    // ...
}
```
3. Most Robust: Add explicit validation and error handling:
```
pub extern "C-unwind" fn r_show_message(buf: *const c_char) {
    if buf.is_valid() {
        // Handle error appropriately
        return;
    }
    // ...
}
```
##AdditionalWhile I understand this is an application rather than a library on crates.io, there are still important reasons to address this issue. Public functions should follow Rust's safety conventions regardless of intended use. And developers working on this codebase in the future might mistakenly use these functions in unsafe ways.

Thank you for considering this issue. I'm happy to provide any additional information that might be helpful.

## @lionel- at 2025-02-28T08:59:58Z

Thank you. We're not interested in changing this signature at the moment. Note that this is a callback invoked by an external C library not by a real programmer.
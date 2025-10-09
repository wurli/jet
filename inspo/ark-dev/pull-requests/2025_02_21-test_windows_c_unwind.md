# Switch from `extern "C"` to `extern "C-unwind"`

> <https://github.com/posit-dev/ark/pull/718>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Closes #678 
Reverts #683 

Joint work with @lionel- 

---

Consider the following

```rust
top_level_exec(|| {
     let msg = CString::new("ouch").unwrap();
     unsafe { Rf_error(msg.as_ptr()) };
})?;
```

This effectively gets called as something similar to this (but I'm oversimplifying a bit):

```rust
extern "C" fn callback(_args: *mut c_void)
{
     let msg = CString::new("ouch").unwrap();
     unsafe { Rf_error(msg.as_ptr()) };
}

unsafe { R_ToplevelExec(Some(callback), std::ptr::null_mut()) };
```

We can also write this in terms of frames

```
[ C: longjmp inside Rf_error ] --------------->
   |                                          |
[ Rust: Rf_error inside callback ]            | Jump over this middle `callback` frame
   |                                          |
[ C: callback inside R_ToplevelExec ]  <------|
   |
[ Rust: R_ToplevelExec inside top_level_exec ]
```

In other words, when a `Rf_error()` causes a `longjmp`, we are forced to jump over the rest of the Rust `callback()` frame. This is _undefined behavior_, even though we immediately catch that longjmp in `R_ToplevelExec()` in the next C frame. The fact that we have jumped over the rest of the Rust `callback()` is simplify undefined behavior, full stop.

The exact result of this undefined behavior seems to be platform dependent. It seems to mostly still work on Unix, as seen by our Mac and Linux builds still working. But on Windows it crashes ark hard with Rust 1.84, and we think it is due to this new `extern "C"` feature where `Drop` methods try to run now https://github.com/rust-lang/rust/pull/129582. In the above example, we think the `Drop` method for `msg` is expected to run, but we've longjmped past `callback()` so it can't properly run, and this blows something up.

---

The best description of this is this table:
https://rust-lang.github.io/rfcs/2945-c-unwind-abi.html#abi-boundaries-and-unforced-unwinding.

<img width="827" alt="Screenshot 2025-02-24 at 10 59 26 AM" src="https://github.com/user-attachments/assets/da5f0ea7-4b6d-4e72-ae53-b6dcfdf665cf" />

We are currently in the combination of `panic=unwind`, `"C"-like`, causing `Unforced foreign unwind` to result in `UB`.

The goal of this PR is to move us to `"C-unwind"`, causing `Unforced foreign unwind` to result in `unwind`. This was introduced in Rust 1.71 https://blog.rust-lang.org/2023/07/13/Rust-1.71.0.html#c-unwind-abi

---

With `"C-unwind`, we still end up jumping over the rest of `callback()`, but Rust now expects that this is possible, so I'm guessing it allows for this internally. So now `msg` is simply "leaked" because its `Drop` method doesn't run (This `Drop` behavior is also platform dependent, notably the `Drop` does seem to still run on Windows, but not on Unix). Before this PR we knew it would be leaked, but now we think we have made the leak "defined behavior" to Rust.

So the moral of the story here is to still be extremely careful with the closure supplied to `try_catch()` and `top_level_exec()`. If the closure longjmps, then you cannot expect any destructors to run, so the closures should only create PODs if possible. But we now believe that jumping over the small `callback()` frame is now well defined.

## @lionel- at 2025-02-25T10:44:02Z

Merging to get these sweet green check marks again!
Thanks for getting at the bottom of this!
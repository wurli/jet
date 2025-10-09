# Move monitoring into `sys/unix/linux` to abstract around the OS details

> <https://github.com/posit-dev/ark/pull/831>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Merges into https://github.com/posit-dev/ark/pull/830

@jmcphers we've done a decent job so far of shoving OS level differences into `sys/` to abstract over the OS specifics at the call sites when we them throughout ark, avoiding `#[cfg(target_os = "linux")]` and friends in the core library

Windows and MacOS just get a stub of the same function that does nothing. This (in theory) makes it easy to expand on in the future if we want.

We've never had a Linux specific one before (up until now, just Unix vs Windows), but I just added `unix/linux` and `unix/macos` subfolders to abstract over that difference too. This nicely lines up with the `target_family` (Windows or Unix) vs `target_os` (Windows or MacOS or Linux or ...) hierarchy:
https://doc.rust-lang.org/reference/conditional-compilation.html#target_family
https://doc.rust-lang.org/reference/conditional-compilation.html#target_os

---

Actual implementation looks fine though!


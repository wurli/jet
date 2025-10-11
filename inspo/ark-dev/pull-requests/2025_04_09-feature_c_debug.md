# Expose helpers for C/C++/Rust debuggers

> <https://github.com/posit-dev/ark/pull/769>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

Closes posit-dev/positron#7174
Progress towards posit-dev/positron#7171

This adds 4 helpers to the global C namespace. The first 3 are part of #7174 and are meant to be directly called by users in the Watch Pane or the Debug Console:

```c
const char* ark_print(SEXP x);
const char* ark_inspect(SEXP x);
const char* ark_trace_back();
```

Here is how it looks when called from the Debug Console:

![Screenshot 2025-04-08 at 12 16 31](https://github.com/user-attachments/assets/948e988c-012b-4a41-afdf-e5c85b7450e4)

 Note you'll need to set this in LLDB otherwise newlines and unicode characters are escaped:

```
settings set escape-non-printables false
```

And here is how it looks in the Watch Pane (by default you need to opt into the complex expression evaluator with `\nat`:

![yay](https://github.com/user-attachments/assets/10a140a2-46d6-4bcd-826d-d23a400b0401)

(This view helped me fix a bug in rlang!)


Worth noting you can also call these from the Rust debugger (e.g. when debugging Ark itself) if you can get your hands on a `SEXP` address.

The last one is meant to be called by formatters for the Variable Pane (posit-dev/positron#7171):

```c
const char* ark_display_value(SEXP x);
```

It is called by the Python formatter implemented in https://github.com/posit-dev/positron/issues/7171#issuecomment-2789145812. Here is how it looks in the Variables Pane:

![Screenshot 2025-04-09 at 12 18 27](https://github.com/user-attachments/assets/d1cfeec8-63d4-4f2a-b38b-5f3244ecf159)


### Visibility of symbols in the debugger

Getting these Ark helpers callable from the debugger was surprisingly hard. Including a function with `"C"` linkage in the binary is not sufficient to be able to call it reliably from the LLDB interpreter. Even though `extern "C"` symbols are visible with `nm`, and even though you can set breakpoints in them, they are not necessarily considered to be in scope by the LLDB interpreter. I've tried `#[no_mangle]` but that didn't help. It seems the only way to make the function globally available is to export it from an actual C file.

So these functions are implemented in `debug.c` and a new directive in `build.rs` gets the compiler to build this new C file. The C file is bare bones and directly calls into Rust implementations.

I also had to trick the Rust compiler into thinking that these functions are needed by the program, otherwise they are treated as dead code and omitted from the Ark binary, even in debug builds. We do this by storing a pointer to a function we'd like to preserve in a global variable and marking this variable with `#[used]` (you can only use that marker on variables, not functions).


### Redirecting output to LLDB

Ideally the output of functions like `Rf_PrintValue` would be redirected to stdout but this is made difficult because by default stdout and stderr are captured and redirected to IOPub again. This allows the output of programs run via `system()` to be sent to the frontend instead of the Ark's process stdout which is invisible to the user.

One way around that is to start Ark with the `--no-capture-streams` argument, then you're able to call `Rf_PrintValue()` and see the output in the console. However it is non obvious to newcomers that this argument exists and even for those who know about it this adds distracting friction to the dev workflow. Also it's easy to forget to unset that argument after debugging which may introduce bugs in regular usage of Ark as C-level output is no longer sent over IOPub.

To avoid having to set `--no-capture-streams` I tried to take a copy of the original file descriptors of stdout/stderr to divert output there. Unfortunately this doesn't seem to work when stopped in lldb, probably because it does some redirection of its own.

The next best thing I could find is to return the output as a C string. If you set this LLDB setting the C string is printed with special characters correctly formatted (newlines in particular):

```
settings set escape-non-printables false
```

The redirection works by flipping a global variable that instructs `RMain::write_console()` to push its input to a buffer instead of sending it over to IOPub. The `capture_console_output()` helper is in charge of setting and restoring this variable.

I took some care to ensure consistency of the state in a debugging context where weird stuff can happen:

- Atomic boolean check in case `write_console()` is being run when the debugger pauses.

- Panics are detected and delayed to allow some time to restore the state.

- R longjumps are also caught.

However I did not make `capture_console_output()` reentrant. If we need to do that we could turn the output buffer into a stack, but that could get complicated so hopefully will not be needed (I can't think of a case where this would be needed).



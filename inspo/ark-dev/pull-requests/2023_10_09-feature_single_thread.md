# Run R tasks on the R thread

> <https://github.com/posit-dev/ark/pull/109>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/1536
Addresses https://github.com/posit-dev/positron/issues/1516
Addresses https://github.com/posit-dev/positron/issues/431
Progress towards https://github.com/posit-dev/positron/issues/1419

The tasks are run one by one at yield/interrupt time. I used a function rather than a macro because we discussed with @DavisVaughan the possibility of implementing his idea of passing the R API as a struct to the callback. This way in files implementing behaviour for auxiliary threads, we'd exclusively access the R API via this struct. In other files implementing behaviour for the main R thread, we could access the R API directly. This delineation will allow us to be more in control of safety.

TODO in further PRs:

- ~Still need to figure out how to send some LSP tasks to the main thread because tree-sitter objects are not Send/Sync. Hopefully we can chop up the tasks more finely. Until then Shiny apps might still crash.~ Edit: now done.

- Add timeout on R tasks. I think this will require longjumping over Rust stacks, but I'll provide some tools to make it possible to reduce the Rust context that will be jumped over.


## @lionel- at 2023-10-10T12:31:07Z

In recent commits I made this change to `r_task()`:

```diff
modified   crates/ark/src/r_task.rs
@@ -25,8 +25,8 @@ type SharedOption<T> = Arc<Mutex<Option<T>>>;
 pub fn r_task<'env, F, T>(f: F) -> T
 where
     F: FnOnce() -> T,
-    F: 'env + Send,
-    T: 'env + Send,
+    F: 'env,
+    T: 'env,
 {
     // Escape hatch for unit tests
     if unsafe { R_TASK_BYPASS } {
@@ -62,7 +62,7 @@ where
         };
 
         // Move `f` to heap and erase its lifetime
-        let closure: Box<dyn FnOnce() + Send + 'env> = Box::new(closure);
+        let closure: Box<dyn FnOnce() + 'env> = Box::new(closure);
         let closure: Box<dyn FnOnce() + Send + 'static> = unsafe { std::mem::transmute(closure) };
 
         // Channel to communicate completion status of the task/closure
```

This goes further than `Crossbeam::thread::ScopedThreadBuilder` from which `r_task()` is adapted. In addition to erasing the closure lifetime and allow it to be sent to another thread (because we guarantee safe storage for the duration of the call), we also erase the `Send` requirement on the variables captured by the closure.

I did this to make it possible to use types that are not `Send` inside the closure. This fixes this sort of compilation errors:

```
rustc [E0277]: `*const c_void` cannot be shared between threads safely
within `completions::CompletionContext<'_>`, the trait `Sync` is not implemented for `*const c_void`
required for `&completions::CompletionContext<'_>` to implement `Send`
rustc [E0277]: `*const tree_sitter::ffi::TSTree` cannot be shared between threads safely
within `completions::CompletionContext<'_>`, the trait `Sync` is not implemented for `*const tree_sitter::ffi::TSTree`
required for `&completions::CompletionContext<'_>` to implement `Send`
rustc [E0277]: required because it's used within this closure
```

I think it is safe to send these objects to another thread because the calling thread is blocked while the task is running, so that we have a perfect delineation of control flow.

Following this change I was able to remove the remaining occurrences of `r_lock!` as well as all the locking machinery. All R API accesses are now executed from the R thread, which addresses https://github.com/posit-dev/positron/issues/1516 and hopefully https://github.com/posit-dev/positron/issues/431.

## @lionel- at 2023-10-10T12:45:18Z

I'm now less convinced we'll be able to make it transparent that R accesses are made on the R thread by passing an `RApi` struct to the task closure. While this would nicely work with free functions, this wouldn't prevent accesses through trait methods since the latter can't really be stored in a struct. Or do you see a way to make it work @DavisVaughan?

## @lionel- at 2023-10-10T13:32:07Z

> I think it is safe to send these objects to another thread because the calling thread is blocked while the task is running, so that we have a perfect delineation of control flow.

hmm we should be safe regarding data races but this does allow sending objects that are sensitive to the thread they are running on. So we expose ourselves to the same sort of problems we ran into on the R side with reticulate and Shiny, only here it's with Rust objects. I think we're fine for now but I'll take another look at solving the `Send` issues without relaxing the safety requirement.

## @lionel- at 2023-10-11T11:35:30Z

> hmm we should be safe regarding data races but this does allow sending objects that are sensitive to the thread they are running on. So we expose ourselves to the same sort of problems we ran into on the R side with reticulate and Shiny, only here it's with Rust objects. I think we're fine for now but I'll take another look at solving the Send issues without relaxing the safety requirement.

We now have `Send` tasks again. This requires dev tree-sitter as well as this branch: https://github.com/r-lib/tree-sitter-r/pull/59. More work is needed to ensure safety, see https://github.com/posit-dev/positron/issues/1550

I also refactored the `read_console()` loop to make control flow clearer (we now match control flow with an evocative enum) and to wake up as soon as a task is available. Waking up this way fixes a performance issue introduced by switching from lock to tasks.
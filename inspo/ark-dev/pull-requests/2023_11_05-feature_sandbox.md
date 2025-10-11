# Run tasks inside top-level context

> <https://github.com/posit-dev/ark/pull/136>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

This PR increases safety and failure reporting of R tasks.

- Adds `r_top_level_exec()`, a wrapper to the correspondingly named function of the R API. Running a closure in a top-level context insulates it from all condition handlers on the stack, which removes one source of longjumps. It also allows us to detect all longjumps of any kinds.

  When a longjump happens, a `TopLevelExecError` is thrown. It contains the contents of R's C-level error buffer as an indication. Note that it is possible for this error message to be unrelated to the actual cause of longjump in edge cases.

- Renames `r_safely()` to `r_sandbox()` which now uses `r_top_level_exec()` internally. Since R tasks are run in a sandbox, we can now detect unexpected errors and longjumps during a task.

  If this happens we log the failure and panic using a new `log_and_panic!` macro. This includes the backtrace of the R thread and the backtrace of the calling thread. This is a panic for now as it would be a substantial task to transform these longjumps into handled errors, as we'd need to consider recovery paths everywhere we call `r_task()`. The current plan is to protect the R calls that might fail with `r_try_catch()` or similar.

For instance for https://github.com/posit-dev/positron/issues/1690 we now get this error:

```
[R] ... While running task: `R_topLevelExec()` error: Unexpected longjump.
[R] Likely caused by: Error: C stack usage  7954072 is too close to the limit
```

And this relevant part of the environment thread backtrace:

```
[R]    4: ark::environment::r_environment::REnvironment::update
[R]              at ./crates/ark/src/environment/r_environment.rs:411:9
[R]    5: ark::environment::r_environment::REnvironment::execution_thread
[R]              at ./crates/ark/src/environment/r_environment.rs:126:25
[R]    6: ark::environment::r_environment::REnvironment::start::{{closure}}
[R]              at ./crates/ark/src/environment/r_environment.rs:98:13
```

Unfortunately we don't get any more precise information on the failing call site since R has longjumped over our Rust stack inside the `update()` task. To prevent this, we need to be more careful about running any potentially failing R code in an `r_try_catch()` context.

Side changes:

- We now implement `fmt::Debug` to report a backtrace if one is available, using the `{:?}` specifier. We used to report the error structure instead but the new reporting is consistent with anyhow errors. To get the error structure as before, use the `{:#?}` specifier.

- The closure types of `r_try_catch()` were relaxed from `FnMut` to `FnOnce` using the same approach I used to implement `r_top_level_exec()`.

- I've added a `rust_backtrace()` helper to our R internal API which I used it to benchmark backtrace creation: 115us. I measured this while I was considering always including a backtrace in every task instead of communicating them on request via channel. I left this helper there as it might be useful later.

- I also added a public `.ps.internal()` helper that makes it easier to access internal tools. For instance you can call the backtrace helper like this: `.ps.internal(rust_backtrace())`.


## @lionel- at 2023-11-05T14:49:50Z

This is currently a panic on the main R thread. We could instead panic on the background thread based on these changes:

```diff
modified   crates/ark/src/r_task.rs
@@ -84,7 +84,7 @@ where
         let closure: Box<dyn FnOnce() + Send + 'static> = unsafe { std::mem::transmute(closure) };

         // Channel to communicate completion status of the task/closure
-        let (status_tx, status_rx) = bounded::<RTaskStatus>(0);
+        let (status_tx, status_rx) = bounded::<harp::error::Result<()>>(0);

         // Send the task to the R thread
         let task = RTaskMain {
@@ -98,12 +98,13 @@ where

         // If the task failed send a backtrace of the current thread to the
         // main thread
-        if let RTaskStatus::Failure(trace_tx) = status {
-            trace_tx.send(std::backtrace::Backtrace::capture()).unwrap();
-
-            // Give some time to main thread to panic
-            std::thread::sleep(Duration::from_secs(5));
-            unreachable!();
+        if let Err(err) = status {
+            let trace = std::backtrace::Backtrace::capture();
+            log_and_panic!(
+                "While running task: {err:?}\n\
+                 Backtrace of calling thread:\n\n\
+                 {trace}"
+            );
         }
     }

@@ -164,12 +165,7 @@ where

 pub struct RTaskMain {
     pub closure: Option<Box<dyn FnOnce() + Send + 'static>>,
-    pub status_tx: Option<Sender<RTaskStatus>>,
-}
-
-pub enum RTaskStatus {
-    Success,
-    Failure(Sender<std::backtrace::Backtrace>),
+    pub status_tx: Option<Sender<harp::error::Result<()>>>,
 }

 impl RTaskMain {
@@ -178,33 +174,19 @@ impl RTaskMain {
         let closure = self.closure.take().unwrap();
         let result = r_sandbox(closure);

-        // Retrieve notification channel of blocking task
-        let status_tx = match &self.status_tx {
-            Some(status_tx) => status_tx,
+        match &self.status_tx {
+            Some(status_tx) => {
+                // Unblock caller via the notification channel if it was a
+                // blocking call. Failures are handled by the caller.
+                status_tx.send(result.map(|_| ())).unwrap()
+            },
             None => {
-                // If task is async return or panic immediately
+                // If task is async panic immediately in case of failure
                 if let Err(err) = result {
                     log_and_panic!("While running task: {err:?}");
                 }
-                return;
             },
         };
-
-        // In case of failure, request backtrace from calling thread and panic
-        if let Err(err) = result {
-            let (trace_tx, trace_rx) = bounded::<std::backtrace::Backtrace>(1);
-            status_tx.send(RTaskStatus::Failure(trace_tx)).unwrap();
-            let trace = trace_rx.recv().unwrap();
-
-            log_and_panic!(
-                "While running task: {err:?}\n\
-                 Backtrace of calling thread:\n\n\
-                 {trace}"
-            );
-        }
-
-        // Unblock caller if it was a blocking call
-        status_tx.send(RTaskStatus::Success).unwrap()
     }
 }
 ```

However with this approach Ark keeps executing in an unstable state. All channels communicating with the panicking threads will propagate the panic for instance.

I think in the long term this is the right approach because it lets users save their work before shutting down their session but it will take quite a bit of work to make sure we properly shut down the background thread, its associated comm, and all communication channels with other threads. In the meantime panicking on the main thread is the more predictable behaviour.

## @lionel- at 2023-11-05T15:47:04Z

Scratch my last message, it was easy to propagate panics from background threads. Since unwinding into non-Rust code is UB, it's better in principle to panic on background threads than on the main thread running R, and that's what we now do. Also the implementation is a bit simpler, and made it easy to add a 5 seconds timeout to tasks. So we now also panic if the task takes longer than 5 seconds.

The latter feature is related to https://github.com/posit-dev/positron/issues/1419 but does not address it. I think to address it we need a recoverable timeout which we'd typically use with a much smaller duration while running foreign R code. The timeout would trigger an interrupt on the R thread that we'd then catch and convert to a Rust error.

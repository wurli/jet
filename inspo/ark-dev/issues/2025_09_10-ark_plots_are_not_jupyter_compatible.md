# ark: Plots are not Jupyter compatible

> <https://github.com/posit-dev/ark/issues/687>
> 
> * Author: @jmcphers
> * State: OPEN
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABSPDwBw", name = "bug", description = "Something isn't working", color = "E99695")

Currently, ark emits `positron.plot` widgets, which are great for displaying in the Plots pane. However, it does not emit plots via `display_data`, so Jupyter front ends -- including VS Code's notebook interface -- can't see them.

This is one of several issues related to getting ark to work properly with non-Positron Jupyter front ends; others:

- https://github.com/rstudio/positron/issues/82
- https://github.com/rstudio/positron/issues/281

## @jmcphers at 2023-05-09T23:46:48Z

An open question we need to solve is how to decide what kind of plot to emit. It is mostly useless to emit static plots to Positron, since it understands dynamic plot widgets. However, it is also mostly useless to create dynamic plot widgets for other front ends.

So -- how do we know if we're connected to Positron or some other front end, so we know what kind of plot output to create?

- A special token included in the session identifier, indicating a Positron session?
- Positron could open a comm with a special ID when connecting to the kernel? We can check to see if this comm is open to know if we are connected to Positron. (I kind of like this idea because it gives us a central place to put custom Positron messages that aren't otherwise associated with a UI widget.)

Alternately, we could just always output every kind of plot, and let the front end sort it out -- Positron will ignore the static bits, and other front ends will ignore the dynamic bits. 

## @petetronic at 2023-05-17T14:34:57Z

This isn't ARK, but if you do need our Python kernel to also send plots via the normal static display_data route, as well as positron.plot, we'll just need to make a tiny change to our PositronDisplayPublisherHook in positron/plots.py. Essentially instead of returning None, we would let the msg return to the caller.
https://github.com/posit-dev/positron-python/blob/main/pythonFiles/positron/plots.py#L85

## @DavisVaughan at 2023-07-12T18:03:43Z

If you open a Jupyter Notebook in VS Code with an ark kernel and try to run `plot(1:100)`, then it should crash immediately. If you attach lldb to the ark process after starting it but before running `plot(1:100)`, you get a backtrace with these notable lines:

```
(lldb) bt
* thread posit-dev/positron#4, name = 'ark-r-main-thread', stop reason = EXC_BREAKPOINT (code=1, subcode=0x10327ea08)
  * frame #0: 0x000000010327ea08 ark`core::option::Option$LT$T$GT$::unwrap_unchecked::hc760db2578e1c75a [inlined] core::hint::unreachable_unchecked::h6a342f5058144008 at intrinsics.rs:2480:9
    frame posit-dev/positron#1: 0x000000010327ea08 ark`core::option::Option$LT$T$GT$::unwrap_unchecked::hc760db2578e1c75a(self=Option<&lock_api::mutex::Mutex<parking_lot::raw_mutex::RawMutex, crossbeam_channel::channel::Sender<amalthea::comm::event::CommEvent>>> @ 0x000000016d591700) at option.rs:1065:30
    frame posit-dev/positron#2: 0x000000010340e71c ark`ark::lsp::globals::comm_manager_tx::h6a0d62866efcd92f at globals.rs:37:14
    frame posit-dev/positron#3: 0x0000000103441370 ark`ark::plots::graphics_device::DeviceContext::new_page::he9af8c88dfe04390(self=0x00000001045f61e0, _dd=0x000000016d591ef8, _dev=0x000000015608a140) at graphics_device.rs:121:31
    frame posit-dev/positron#4: 0x0000000103443338 ark`ark::plots::graphics_device::gd_new_page::h59b28c852b9261ff(dd=0x000000016d591ef8, dev=0x000000015608a140) at graphics_device.rs:318:5
```

- `new_page()` is calling `comm_manager_tx()` here: https://github.com/posit-dev/amalthea/blob/7ccaa2678e79a2a49ecb7420b8ec4db533404171/crates/ark/src/plots/graphics_device.rs#L121-L123
- But `comm_manager_tx()` is part of the LSP. It requires `COMM_MANAGER_TX` to be initialized
- `COMM_MANAGER_TX` is initialized by `lsp::globals::initialize()`
- Which is only called by `start_lsp()`
- Which is only called by the LSP Comm
- But the LSP Comm is only ever started by this special `Comm::Lsp` request that I think only Positron sends https://github.com/posit-dev/amalthea/blob/7ccaa2678e79a2a49ecb7420b8ec4db533404171/crates/amalthea/src/socket/shell.rs#L417

So `comm_manager_tx()` segfaults here because `COMM_MANAGER_TX` isn't initialized

<details>

```
(lldb) bt
* thread posit-dev/positron#4, name = 'ark-r-main-thread', stop reason = EXC_BREAKPOINT (code=1, subcode=0x10327ea08)
  * frame #0: 0x000000010327ea08 ark`core::option::Option$LT$T$GT$::unwrap_unchecked::hc760db2578e1c75a [inlined] core::hint::unreachable_unchecked::h6a342f5058144008 at intrinsics.rs:2480:9
    frame posit-dev/positron#1: 0x000000010327ea08 ark`core::option::Option$LT$T$GT$::unwrap_unchecked::hc760db2578e1c75a(self=Option<&lock_api::mutex::Mutex<parking_lot::raw_mutex::RawMutex, crossbeam_channel::channel::Sender<amalthea::comm::event::CommEvent>>> @ 0x000000016d591700) at option.rs:1065:30
    frame posit-dev/positron#2: 0x000000010340e71c ark`ark::lsp::globals::comm_manager_tx::h6a0d62866efcd92f at globals.rs:37:14
    frame posit-dev/positron#3: 0x0000000103441370 ark`ark::plots::graphics_device::DeviceContext::new_page::he9af8c88dfe04390(self=0x00000001045f61e0, _dd=0x000000016d591ef8, _dev=0x000000015608a140) at graphics_device.rs:121:31
    frame posit-dev/positron#4: 0x0000000103443338 ark`ark::plots::graphics_device::gd_new_page::h59b28c852b9261ff(dd=0x000000016d591ef8, dev=0x000000015608a140) at graphics_device.rs:318:5
    frame posit-dev/positron#5: 0x0000000106d4b0a8 graphics.so`Rf_GNewPlot + 296
    frame posit-dev/positron#6: 0x0000000106d58770 graphics.so`C_plot_new + 48
    frame posit-dev/positron#7: 0x000000010671351c libR.dylib`do_External(call=0x0000000155ddbc38, op=0x000000015682bdb8, args=0x0000000155ddf728, env=0x0000000155ddb420) at dotcode.c:573:11 [opt]
    frame posit-dev/positron#8: 0x000000010674aa24 libR.dylib`bcEval(body=0x0000000155ddc1e8, rho=<unavailable>, useCache=<unavailable>) at eval.c:7446:14 [opt]
    frame posit-dev/positron#9: 0x0000000106743048 libR.dylib`Rf_eval(e=0x0000000155ddc1e8, rho=0x0000000155ddb420) at eval.c:1013:8 [opt]
    frame posit-dev/positron#10: 0x000000010675fccc libR.dylib`R_execClosure(call=0x0000000155dbcfe8, newrho=0x0000000155ddb420, sysparent=<unavailable>, rho=<unavailable>, arglist=<unavailable>, op=0x0000000155ddc530) at eval.c:0 [opt]
    frame posit-dev/positron#11: 0x000000010675e54c libR.dylib`Rf_applyClosure(call=0x0000000155dbcfe8, op=0x0000000155ddc530, arglist=0x000000015680dee0, rho=0x0000000157010070, suppliedvars=<unavailable>) at eval.c:2113:16 [opt]
    frame posit-dev/positron#12: 0x000000010674a204 libR.dylib`bcEval(body=0x0000000155dbac90, rho=<unavailable>, useCache=<unavailable>) at eval.c:7414:12 [opt]
    frame posit-dev/positron#13: 0x0000000106743048 libR.dylib`Rf_eval(e=0x0000000155dbac90, rho=0x0000000157010070) at eval.c:1013:8 [opt]
    frame posit-dev/positron#14: 0x000000010675fccc libR.dylib`R_execClosure(call=0x0000000155dbebc0, newrho=0x0000000157010070, sysparent=<unavailable>, rho=<unavailable>, arglist=<unavailable>, op=0x0000000156c4a710) at eval.c:0 [opt]
    frame posit-dev/positron#15: 0x000000010675e54c libR.dylib`Rf_applyClosure(call=0x0000000155dbebc0, op=0x0000000156c4a710, arglist=0x0000000156c4af60, rho=0x0000000156c4ae80, suppliedvars=<unavailable>) at eval.c:2113:16 [opt]
    frame posit-dev/positron#16: 0x00000001067ae974 libR.dylib`applyMethod(call=<unavailable>, op=<unavailable>, args=<unavailable>, rho=<unavailable>, newvars=<unavailable>) at objects.c:118:8 [opt] [artificial]
    frame posit-dev/positron#17: 0x00000001067ad2c0 libR.dylib`dispatchMethod(op=0x0000000156c4b190, sxp=0x0000000156c4a710, dotClass=0x000000015680dee0, cptr=0x000000016d594a70, method=0x0000000157e016f8, generic="plot", rho=0x0000000156c4ae80, callrho=0x0000000156846d88, defrho=0x0000000156846ce0) at objects.c:399:16 [opt]
    frame posit-dev/positron#18: 0x00000001067acf7c libR.dylib`Rf_usemethod(generic="plot", obj=<unavailable>, call=<unavailable>, args=<unavailable>, rho=0x0000000156c4ae80, callrho=0x0000000156846d88, defrho=0x0000000156846ce0, ans=0x000000016d594150) at objects.c:0 [opt]
    frame posit-dev/positron#19: 0x00000001067ad594 libR.dylib`do_usemethod(call=0x0000000156c4b078, op=<unavailable>, args=<unavailable>, env=0x0000000156c4ae80) at objects.c:505:9 [opt]
    frame posit-dev/positron#20: 0x000000010674abfc libR.dylib`bcEval(body=0x0000000156c4b0b0, rho=<unavailable>, useCache=<unavailable>) at eval.c:7466:15 [opt]
    frame posit-dev/positron#21: 0x0000000106743048 libR.dylib`Rf_eval(e=0x0000000156c4b0b0, rho=0x0000000156c4ae80) at eval.c:1013:8 [opt]
    frame posit-dev/positron#22: 0x000000010675fccc libR.dylib`R_execClosure(call=0x0000000156c4b3f8, newrho=0x0000000156c4ae80, sysparent=<unavailable>, rho=<unavailable>, arglist=<unavailable>, op=0x0000000156c4b190) at eval.c:0 [opt]
    frame posit-dev/positron#23: 0x000000010675e54c libR.dylib`Rf_applyClosure(call=0x0000000156c4b3f8, op=0x0000000156c4b190, arglist=0x0000000156c4af60, rho=0x0000000156846d88, suppliedvars=<unavailable>) at eval.c:2113:16 [opt]
    frame posit-dev/positron#24: 0x000000010674331c libR.dylib`Rf_eval(e=0x0000000156c4b3f8, rho=0x0000000156846d88) at eval.c:1140:12 [opt]
    frame posit-dev/positron#25: 0x00000001067973b4 libR.dylib`Rf_ReplIteration(rho=0x0000000156846d88, savestack=<unavailable>, browselevel=<unavailable>, state=0x000000016d594fd0) at main.c:262:2 [opt]
    frame posit-dev/positron#26: 0x0000000106798928 libR.dylib`R_ReplConsole(rho=0x0000000156846d88, savestack=0, browselevel=0) at main.c:314:11 [opt]
    frame posit-dev/positron#27: 0x0000000106798864 libR.dylib`run_Rmainloop at main.c:1200:5 [opt]
    frame posit-dev/positron#28: 0x00000001032a4a34 ark`ark::interface::start_r::hc39b4358c6979ff5(kernel_mutex=Arc<std::sync::mutex::Mutex<ark::kernel::Kernel>> @ 0x000000016d596280, r_request_rx=(flavor = crossbeam_channel::channel::ReceiverFlavor<> @ 0x000000016d596288), input_request_tx=(flavor = crossbeam_channel::channel::SenderFlavor<> @ 0x000000016d596298), iopub_tx=(flavor = crossbeam_channel::channel::SenderFlavor<> @ 0x000000016d5962a8), kernel_init_tx=<unavailable>) at interface.rs:214:9
    frame posit-dev/positron#29: 0x0000000103089820 ark`ark::shell::Shell::new::_$u7b$$u7b$closure$u7d$$u7d$::h544b9afa0fe899e3 at shell.rs:103:13
    frame posit-dev/positron#30: 0x000000010321e478 ark`std::sys_common::backtrace::__rust_begin_short_backtrace::h85d5b3b27342cc94(f=<unavailable>) at backtrace.rs:134:18
    frame posit-dev/positron#31: 0x000000010302f934 ark`std::thread::Builder::spawn_unchecked_::_$u7b$$u7b$closure$u7d$$u7d$::_$u7b$$u7b$closure$u7d$$u7d$::haa5fd9d8a47068cf at mod.rs:526:17
    frame posit-dev/positron#32: 0x0000000102f0f204 ark`_$LT$core..panic..unwind_safe..AssertUnwindSafe$LT$F$GT$$u20$as$u20$core..ops..function..FnOnce$LT$$LP$$RP$$GT$$GT$::call_once::h607162b75247cef4(self=<unavailable>, (null)=<unavailable>) at unwind_safe.rs:271:9
    frame posit-dev/positron#33: 0x0000000102f8dbd4 ark`std::panicking::try::do_call::hfd405c423bd69f15(data="\U00000002") at panicking.rs:485:40
    frame posit-dev/positron#34: 0x0000000102f94024 ark`__rust_try + 32
    frame posit-dev/positron#35: 0x0000000102f89ddc ark`std::panicking::try::hfa3d1ed75e2a2629(f=<unavailable>) at panicking.rs:449:19
    frame posit-dev/positron#36: 0x000000010304697c ark`std::panic::catch_unwind::h8624920d6f209534(f=<unavailable>) at panic.rs:140:14
    frame posit-dev/positron#37: 0x000000010302e9f0 ark`std::thread::Builder::spawn_unchecked_::_$u7b$$u7b$closure$u7d$$u7d$::hba9d1e795e0cfb6a at mod.rs:525:30
    frame posit-dev/positron#38: 0x00000001030cdfdc ark`core::ops::function::FnOnce::call_once$u7b$$u7b$vtable.shim$u7d$$u7d$::h41c055237ba24155((null)=0x0000000154709ef0, (null)=<unavailable>) at function.rs:250:5
    frame posit-dev/positron#39: 0x0000000103d2d4fc ark`std::sys::unix::thread::Thread::new::thread_start::h7f56b35fafcfec87 [inlined] _$LT$alloc..boxed..Box$LT$F$C$A$GT$$u20$as$u20$core..ops..function..FnOnce$LT$Args$GT$$GT$::call_once::hd96d02f907263858 at boxed.rs:1973:9 [opt]
    frame posit-dev/positron#40: 0x0000000103d2d4f0 ark`std::sys::unix::thread::Thread::new::thread_start::h7f56b35fafcfec87 [inlined] _$LT$alloc..boxed..Box$LT$F$C$A$GT$$u20$as$u20$core..ops..function..FnOnce$LT$Args$GT$$GT$::call_once::hfd80494da4543cfb at boxed.rs:1973:9 [opt]
    frame posit-dev/positron#41: 0x0000000103d2d4ec ark`std::sys::unix::thread::Thread::new::thread_start::h7f56b35fafcfec87 at thread.rs:108:17 [opt]
    frame posit-dev/positron#42: 0x000000019ef27fa8 libsystem_pthread.dylib`_pthread_start + 148
```

</details>

## @DavisVaughan at 2023-07-14T14:33:31Z

Do we know how something like Jupyter Notebooks send requests for plots to be rendered?

In Positron we seem to send that request here:
https://github.com/rstudio/positron/blob/2b90a899b91602e0029561df2e8676a467c9b815/src/vs/workbench/services/languageRuntime/common/languageRuntimePlotClient.ts#L359-L370

Which is then picked up around here on the Rust side, causing us to actually run `render_plot()`:
https://github.com/posit-dev/amalthea/blob/1f2381549810166f75fbee6624ccbd77893367bd/crates/ark/src/plots/graphics_device.rs#L156-L174

But in a Jupyter Notebook we never actually get a "plot render request" so this `select()` always results in an early exit
https://github.com/posit-dev/amalthea/blob/1f2381549810166f75fbee6624ccbd77893367bd/crates/ark/src/plots/graphics_device.rs#L137-L147

## @kevinushey at 2023-07-14T18:22:39Z

It looks like the Jupyter protocol tries to handle this with `display_data` and `update_display_data`: https://jupyter-client.readthedocs.io/en/stable/messaging.html#display-data

## @DavisVaughan at 2023-07-14T18:28:16Z

Yea I got that far and I can stick an IOPub `display_data` message in `render_plot()`, but I'm having trouble getting `render_plot()` to fire at all since a "plot render request" doesn't seem to be triggered by anything other than Positron itself

## @kevinushey at 2023-07-14T18:44:53Z

Ah, I think I see what you mean...

Right now, whenever a new plot page is generated, we notify Positron here:

https://github.com/posit-dev/amalthea/blob/af701b0f8ac0b724d95d047234cb79d8d5653357/crates/ark/src/plots/graphics_device.rs#L108-L128

I wonder if that needs some check like "if we're in Jupyter, send a display_data request instead", or something similar? But I'm not sure if or how these messages need to be orchestrated to be seen as part of the execution result for a particular cell.

## @DavisVaughan at 2023-08-10T20:42:41Z

With https://github.com/posit-dev/amalthea/pull/73 we now emit jupyter compatible plots, but they don't respect any user specified sizing options (they are currently just hardcoded). We should probably get this from the user through an R global option like `options(ark.plots.height = 400)` or something.

## @DavisVaughan at 2025-06-03T18:58:27Z

User request in https://stackoverflow.com/questions/79651489/control-the-size-of-the-output-plot-in-r-notebook-in-positron

## @juliasilge at 2025-06-13T16:14:01Z

I believe we have a related issue at https://github.com/posit-dev/positron/issues/8104.

## @juliasilge at 2025-09-10T15:12:06Z

We've got a request for user specified sizing options in https://github.com/posit-dev/positron/issues/9378.
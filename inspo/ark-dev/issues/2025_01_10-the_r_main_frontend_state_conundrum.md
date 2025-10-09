# The `R_MAIN` frontend state conundrum

> <https://github.com/posit-dev/ark/issues/663>
> 
> * Author: @lionel-
> * State: OPEN
> * Labels: 

The stable Rust compiler has started to warn about Ark taking references to static mut variables (see #661). The trickiest instance of this concerns `R_MAIN`, our global singleton for the state used by R's frontend callbacks: https://github.com/posit-dev/ark/blob/1366044e69062bc88a8d1bcc6f474c980f6f8166/crates/ark/src/interface.rs#L158. I attempted to fix it in #662 but drove myself into a dead end. This is a hard problem that we'll have to fix later if we can (and if it's worth the work, given that the current situation seemingly works without any obvious problem).

The current situation is kinda bad in terms of Rust safety. `R_MAIN` is a `static mut`, so a mutable global variable. It is accessed mainly by R's frontend callbacks such as `ReadConsole`, `WriteConsole`, or `PolledEvents`, as well as by some of our R-level functions, e.g. via `.ps.Call("ps_browse_url", url)`. Since these callbacks come from C and go through FFI, we must retrieve our `RMain` singleton from global state. R does not pass any frontend-provided state to these callbacks, and even if it did it would be through a raw pointer, which would be equivalent to what we have now (minus some undefined behaviour which I'll get into below). These callbacks access the global singleton via `RMain::get()` or `RMain::get_mut()`: https://github.com/posit-dev/ark/blob/1366044e69062bc88a8d1bcc6f474c980f6f8166/crates/ark/src/interface.rs#L598.

The problem here is that we're completely bypassing the borrow checker because Rust is not able to check how we take references to global `static mut` variables like `R_MAIN`. This is why this action is being deprecated/discouraged by the compiler. But not only are we on our own in terms of ensuring safety, we're also violating the terms of the language. As mentioned in https://doc.rust-lang.org/nightly/edition-guide/rust-2024/static-mut-references.html, taking a `&mut` in violation of Rust reference rules (there can't be any other reference to an object if there exists a `&mut` to it) is instant undefined behaviour. And that's exactly what we're currently doing without realising it.

For instance, `write_console()` takes a `&mut` on `R_MAIN` here: https://github.com/posit-dev/ark/blob/1366044e69062bc88a8d1bcc6f474c980f6f8166/crates/ark/src/interface.rs#L1602. Then we call the helper `is_auto_printing()` here: https://github.com/posit-dev/ark/blob/1366044e69062bc88a8d1bcc6f474c980f6f8166/crates/ark/src/interface.rs#L1622. It so happens that this function evaluates R code (not directly, you have to dig deep to find that out). We haven't disabled polled events, so when R checks for interrupts, `polled_events()`, another of our callbacks, is invoked _while `write_console()` is running_: https://github.com/posit-dev/ark/blob/1366044e69062bc88a8d1bcc6f474c980f6f8166/crates/ark/src/interface.rs#L1709. This callback also takes a `&mut` on `R_MAIN` in violation of Rust rules.

So there are two issues here:

1. The violation of rust borrowing rules.
2. The inherent unsafety of bypassing the borrow checker.

Fixing (1) is easy. We just need to reference `R_MAIN` with a raw pointer. And that's what we'll do for now. But this solution doesn't resolve (2).

In #662 I attempted to fix both (1) and (2). One of the big appeal of Rust is its safety in terms of data invalidation so it is unfortunate to have the core of Ark completely unchecked. I tried two approaches that both involve `RefCell`. A `RefCell` moves the borrow checking from compilation-time to runtime. If a violation is detected, that's a panic. That's not a great outcome but at least we'll instantly know (or users will if not covered by our tests...) if our assumptions about data accesses are wrong.

The first approach I tried quickly proved to be unworkable. The idea was to wrap `R_MAIN` itself in a `RefCell` and access it as before. But that's very dangerous. Think of the `polled_events()` invokation during `write_console()` I mentioned above. This causes a panic because `write_console()` needs a `&mut` and `polled_events()` also needs a reference. In this particular case, it was possible to work around this by reorganising the code, but it felt extremely brittle. We just can't expose ourselves to panics in this way.

So I thought I'm going to make `R_MAIN` immutable and modify its state with _nested_ `RefCell`s, using interior mutability (https://doc.rust-lang.org/book/ch15-05-interior-mutability.html). The WIP which I quickly abandoned is in https://github.com/posit-dev/ark/pull/662/commits/d7ae71acafbd3f2ba1404cebfa6528546006b8ba. The problem is that this approach requires _extensive_ and ultimately brittle changes in the codebase, introducing `RefCell` in many places. First these changes add a lot of verbosity due to the need to `.borrow()` or `.borrow_mut()` for all accesses to mutable objects. Second, while reasoning about the borrowing rules to make sure there is only one `borrow_mut()` at any time is now easier in some ways, because the mutable state is more local, it's also harder in other ways, because any helper you call might possibly borrow the state you're interested in, possibly mutably. To avoid this you should hold references to `RefCell` for the shortest amount of time possible. But while I was in the midst of doing these changes, it felt very brittle as well.

It does feel like something is wrong in our design and we shouldn't need so many `RefCell`. I can think of the following alternative:

1. Divide the `RMain` singleton into multiple singletons in charge of a particular callback or set of callbacks. The singletons would communicate via message-passing if needed.

2. Possibly move some of the remaining `RMain` state to another thread that would receive events from the other singletons running on the R thread via channel.

However that would be a very large undertaking with a rather theoretical payoff.



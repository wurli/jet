# Implement `libr` crate for dynamic runtime symbol resolution

> <https://github.com/posit-dev/ark/pull/205>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Joint PR with https://github.com/posit-dev/positron/pull/2083

I have confirmed that this works on manually generated release builds for both macOS (arm64) and Windows

---

There are two main goals of this PR:
- Remove the libR-sys crate, which used _load time dynamic linking_
- Replace it with our own libr crate, which uses _run time dynamic linking_

This has many benefits, including:
- _Compiling_ ark no longer requires an R dependency (which came from libR-sys). _Running_ ark requires the `R_HOME` env var to be set. (This simplifies the development experience, especially on Windows, where we had to remember to set `R_HOME` before compiling ark if we also had to compile libR-sys.)

- The above point also simplifies our CI scripts considerably, as we don't have to install R and set `R_HOME` to build ark.

- On Windows, we don't need a `R.lib` file at build time, so we don't need the `dll2lib.R` script (simplifies CI build script and development experience on Windows, where you had to run that script at least 1 time per R version you wanted to use)

- On Windows, we no longer need to find `R.dll` at load time. This means we don't need to add the folder containing `R.dll` to the `PATH` in positron-r (which I considered to be a little fragile too). We still need `R.dll` at run time, but we navigate to it relative to `R_HOME` from within ark itself.

- On macOS, we no longer need to find `libR.dylib` at load time (same rationale as Windows). This means we no longer need to set `DYLIB_FALLBACK_LIBRARY_PATH`, which required some shenanigans anyways due to SIP.

- We now fully control our R API bindings, which is going to be critical for long term support of older R versions (i.e. if libR-sys decides they no longer support our oldest R version, we were going to be in a bad place). It is also nice because libR-sys wasn't exposing everything that an IDE needed (their target user was a package developer), so we were already maintaining a small set of extra bindings.

- We now have the option to easily inject debug only code into our R API wrappers (for functions and mutable globals, not constant globals), such as "panic if this was called off the main R thread", which is going to be extremely useful for debugging.

- We can now easily handle cases where we'd _prefer_ to call a "new" R API function, which doesn't exist in older R versions, using the `libr::has::Rf_*()` namespace (for example, `R_existsVarInFrame()` and `R_DefParamsEx()`, which are R >= 4.2.0).

- We have the ability to mark R API bindings as platform specific with `#[cfg(target_family = "windows")]`, which prevents the binding from showing up in `libr::` or `libr::has::` at all when you aren't on that platform, which is quite nice.

- On Windows, the global variables exported by libR-sys didn't work out of the box for us due to them missing the `#[link(name = "R", kind = "dylib")]` attribute. This required us to add our own libR-shim wrapper crate, which is no longer needed.

- On macOS, we no longer need to set `-undefined dynamic_lookup`. This was necessary in some cases because we might build ark against R 4.0.0, but we wanted to be able to use an R 4.2.0 function if the user _loaded_ ark with that R version. We no longer build ark against a specific R version, and all of this is done at runtime now, so it isn't needed.

- On macOS, we no longer need the `post-install.sh` script that would use `install_name_tool` to fix up `@rpath/libR.dylib` so it was dynamic to whatever R version the user loaded ark with. This simplifies our `CMD + SHIFT + B` binding a little too, since now it aligns with what we do on Windows (i.e. just `cargo build`)

## @lionel- at 2024-01-18T15:20:50Z

Amazing!

## @lionel- at 2024-01-18T16:05:48Z

This also makes it much easier to use ark with other Jupyter frontends, basically removing any hacks! I guess we run the R on the `PATH` to get a home if `R_HOME` is not set?

## @DavisVaughan at 2024-01-18T16:28:05Z

> This also makes it much easier to use ark with other Jupyter frontends, basically removing any hacks! I guess we run the R on the PATH to get a home if R_HOME is not set?

That currently only happens in the _testing_ version of `start_r()`. (in the testing CI we don't set `R_HOME` right now, for whatever reason)

In the "real" version of `start_r()`, we 100% expect `R_HOME` to be set, and panic if it isn't set.

I guess we could rethink that though and try to unify those!

## @jennybc at 2024-01-18T16:42:48Z

This is so beautiful 🤩 

## @jmcphers at 2024-01-18T17:21:54Z

This is AMAZING

## @DavisVaughan at 2024-01-23T16:00:28Z

> I think it'd be very useful to retrieve R_HOME from the R found in PATH, if not already set. This will make the usage of Ark as a Jupyter kernel easy and frictionless.

Extracted into https://github.com/posit-dev/positron/issues/2111

## @DavisVaughan at 2024-01-23T16:06:17Z

> That said, we did discuss the idea of using API struct generated on the R main thread and that would be passed around to task lambdas to ensure that no other threads can call into R. So maybe we should still consider that for safety sake

We can also consider adding some debug only code into the macro (if we kept it), which would panic when an r function is called if we aren't on the main thread. I figured we could leave that as a follow up PR.

## @DavisVaughan at 2024-01-23T17:25:00Z

> For instance there is a --dynamic-loading 

I did look at this, but it doesn't support global variables. There was a PR but it was never merged. I'm also remembering some other reason I didn't want to use this, but can't currently remember why (maybe it had something to do with platform specific or R version specific bindings) https://github.com/rust-lang/rust-bindgen/pull/2114

## @DavisVaughan at 2024-01-23T17:27:28Z

Weird, I don't see the `crate::sys::types::*` unused import warning. On mac / linux, the types.rs file is currently empty, so that is probably why you see it, but I figured that would "just work" anyways

## @lionel- at 2024-01-23T18:14:19Z

> There was a PR but it was never merged. 

It was merged. In their workflow they merge the PR branch outside of the github UI and then close the PR.

## @lionel- at 2024-01-23T18:19:31Z

> I did look at this, but it doesn't support global variables. 

Right. I think the get/set approach you just discussed above would probably be a good fit?

> maybe it had something to do with platform specific or R version specific bindings

This seems like an issue of how to use the tool rather than of the tool itself?


## @DavisVaughan at 2024-01-23T18:27:02Z

Hmm, I don't see any support for it here
https://github.com/rust-lang/rust-bindgen/blob/main/bindgen/codegen/dyngen.rs

And this issue stayed open https://github.com/rust-lang/rust-bindgen/issues/2113

## @lionel- at 2024-01-23T18:34:54Z

Sorry I misunderstood, I thought you were talking about the `--dynamic-loading` PR.

## @lionel- at 2024-01-24T08:36:17Z

Note that sourcing `etc/ldpaths` to set the R libs in `LD_LIBRARY_PATH` (which I agree is a good idea) will not be sufficient to reproduce the linking behaviour with our `dlopen`-ed R. To fully reproduce it, we need to set `RTLD_GLOBAL` as this will also expose the transitive dependencies of `libR.so` to subsequently dynloaded libraries. These indirect dependencies might be stored anywhere, not just in `R_HOME/lib`. If we use `RTLD_LOCAL` and R dynloads a module that also depends on one of these transitive dependencies, the linker will try to find them using the current state of the search path and there is a risk of ending up linking against different versions of a library, resulting in duplicates (in possibly inconsistent state) and perhaps version mismatches.

The bottom line is that when we link to `libR.so` the normal way, the dynamic loader uses `RTLD_GLOBAL` behaviour and I think we want to reproduce that with `dlopen()`. Not that it's necessarily the ideal behaviour but to remove a source of variation between Ark and other R frontends.

## @DavisVaughan at 2024-01-24T15:08:10Z

This is the best description of `RTLD_GLOBAL` that I can find:
https://stackoverflow.com/questions/70198803/linux-and-shared-libraries-linking-vs-dlopen-symbol-visibility

<blockquote>
If you `dlopen` the library, then about the only way to get to _any_ of its symbols is via `dlsym`.


However, if you `dlopen` a library with `RTLD_GLOBAL`, then its symbols become available for _subsequently_ loaded libraries _without_ using `dlsym`.

For example, if `libfoo.so` defines symbol `foo`, and if you `dlopen("libfoo.so", RTLD_GLOBAL|...);` and later `dlopen("libbar.so", ...)` which _uses_ `foo`, that would work -- `libbar.so` will be able to use `foo` from `libfoo.so` without doing any `dlsym` calls.
</blockquote>

That seems...super dangerous?? Like, the risk of symbol clash goes up dramatically in that case. Especially since R exposes symbols like the very generic `error()`.

I'll be honest, I haven't seen a single post of anyone _advocating_ for the use of `RTLD_GLOBAL`. All I see are reports of issues that result from it (some kind of symbol clash), which are eventually solved by removing it:
- https://github.com/pytorch/pytorch/issues/3059
- https://github.com/tensorflow/tensorflow/commit/5c7f9e31

---

I see these libR dependencies

```
otool -L /Library/Frameworks/R.framework/Versions/4.4-arm64/Resources/lib/libR.dylib
/Library/Frameworks/R.framework/Versions/4.4-arm64/Resources/lib/libR.dylib:
	/Library/Frameworks/R.framework/Versions/4.4-arm64/Resources/lib/libR.dylib (compatibility version 4.4.0, current version 4.4.0)
	/Library/Frameworks/R.framework/Versions/4.4-arm64/Resources/lib/libRblas.dylib (compatibility version 0.0.0, current version 0.0.0)
	/Library/Frameworks/R.framework/Versions/4.4-arm64/Resources/lib/libgfortran.5.dylib (compatibility version 6.0.0, current version 6.0.0)
	/Library/Frameworks/R.framework/Versions/4.4-arm64/Resources/lib/libquadmath.0.dylib (compatibility version 1.0.0, current version 1.0.0)
	/System/Library/Frameworks/CoreFoundation.framework/Versions/A/CoreFoundation (compatibility version 150.0.0, current version 1775.118.101)
	/usr/lib/libncurses.5.4.dylib (compatibility version 5.4.0, current version 5.4.0)
	/usr/lib/libbz2.1.0.dylib (compatibility version 1.0.0, current version 1.0.5)
	/usr/lib/libz.1.dylib (compatibility version 1.0.0, current version 1.2.11)
	/usr/lib/libicucore.A.dylib (compatibility version 1.0.0, current version 66.1.0)
	/usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version 1292.100.5)
```

If we ensure that R's `/lib/` path is in `LD_LIBRARY_PATH`, then to me it seems reasonable for the other ones to just always be looked up by the standard means. That feels similar to what we do on Windows (i.e. pre open the R specific DLLs so other packages can find them, but let anything else just be looked up with the normal method).

I have no idea what kind of symbols are in, for example, libicucore, but I don't think I would want to globally expose them for any packages to accidentally use or collide with

---

Out of curiosity, does setting `LD_LIBRARY_PATH` seem to work?

## @lionel- at 2024-01-24T17:29:10Z

In general you want to isolate modules/plugins as much as possible, which is why you'll find advice to use `RTLD_LOCAL`. However in our case libR is like our main application. I think the main reason to use `RTLD_GLOBAL` is that it reproduces what happens when you link your application to `libR.so` at load time rather than runtime. That will remove a source of potential subtle differences in symbol resolution in edge cases (e.g. packages that dynloads some specialty software on some linux server somewher) that would be quite hard to debug.

## @DavisVaughan at 2024-01-24T22:44:45Z

Okay, as of the last few commits we now:

- Have `get()` and `set()` for mutable globals, like `libr::set(R_interactive, Rboolean_TRUE)`
- Have fixed the `crate::sys::types::*` unused import warning
- Open `libR` on macOS and Linux with `RTLD_GLOBAL`, with lots of documentation about this
- Source `ldpaths`, if it exists, to set `DYLD_FALLBACK_LIBRARY_PATH` or `LD_LIBRARY_PATH` depending on the OS. Since `ldpaths` itself doesn't use a shebang, we source the file into a bash session with `bash -c '. /path/to/etc/ldpaths'` and then echo out the relevant env var that ldpaths exported

@lionel- can you please check if this works for you, and that the last few commits look okay to you? Then I'll merge.

## @lionel- at 2024-01-25T10:56:08Z

@DavisVaughan On Windows the `ldpaths` file doesn't seem to exist but could we try implementing a similar setup by adding `R_HOME/bin/x64` to `PATH` with `env::set_var()` prior to loading R? Then we should be able to remove this part on the positron-r side:

```
// On Windows, we must place the `bin/` path for the current R version on the PATH
// so that the DLLs in that same folder can be resolved properly when ark starts up
// (like `R.dll`, `Rblas.dll`, `Rgraphapp.dll`, `Riconv.dll`, and `Rlapack.dll`).
```

And this should also (hopefully) make jupyter usage of Ark "just work" on Windows.

## @DavisVaughan at 2024-01-25T14:04:26Z

@lionel- on Windows we do something else in this PR to solve for this. (Kevin and I talked about this one)

```
        // On Windows, we preemptively open the supporting R DLLs that live in
        // `bin/x64/` before starting R. R packages are allowed to link to these
        // DLLs, like stats, and they must be able to find them when the packages
        // are loaded. Because we don't add the `bin/x64` folder to the `PATH`,
        // we instead open these 4 DLLs preemptively and rely on the fact that the
        // "Loaded-module list" is part of the standard search path for dynamic link
        // library searching.
        // https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-search-order
```

Look for harp's `sys/windows/library.rs` file
https://github.com/posit-dev/amalthea/pull/205/files#diff-7cebefab7646da86f0abc45b2ea962bc2e2a634e6f6b94ecd87171dd85401306R32

And I cleaned that positron-r stuff up in this companion PR
https://github.com/posit-dev/positron/pull/2083

## @lionel- at 2024-01-25T14:14:55Z

oh nice! Sorry I forgot about that other PR.
# Disabling interactivity for `r_task`

> <https://github.com/posit-dev/ark/pull/238>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

If a front-end triggered `r_task` requests user input with `readline()` we currently `panic` due to:

https://github.com/posit-dev/amalthea/blob/33351343592c1cdbee3b3d923ef923a862208088/crates/ark/src/interface.rs#L645-L647

This PR disables interactivity when executing `r_task`s making it slightly safer when front-end code might cause `readline()` to be executed.

## @lionel- at 2024-02-13T14:06:13Z

@dfalbel This requires a bit more work for Windows support. In https://github.com/posit-dev/amalthea/actions/runs/7887438579/job/21522697083 I see:

```r
error[E0432]: unresolved import `libr::R_Interactive`
  --> crates\ark\src\r_task.rs:18:5
   |
18 | use libr::R_Interactive;
   |     ^^^^^^^^^^^^^^^^^^^ no `R_Interactive` in the root
```

It looks like changing Windows globals requires calling `R_SetParams` (see sys/windows/interface.rs). We currently don't seem to keep track of the `Rstart` struct so that might need to be changed to a global.

@DavisVaughan Do you have any insights regarding global poking on Windows?

## @DavisVaughan at 2024-02-13T14:33:57Z

I _think_ we may actually be able to relax the unix specific import here, as I think `R_Interactive` is available on Windows too. I'll check and make a PR if that works on the VM
https://github.com/wch/r-source/blob/575a90c3588f0cf419a3a0cea354833078bcd6e9/src/include/Defn.h#L1490

## @DavisVaughan at 2024-02-13T18:16:07Z

Okay, reporting back. `R_Interactive` _is_ specific to Unix.

I investigated both RStudio's source code and R's source code. Neither of them set `R_Interactive` at any point except during startup. I don't really think you are intended to flip back and forth between interactive / non-interactive within a single R session.

I think we should just try hard to avoid code that can call `readline()` in an `r_task()`. I'm not sure of a better thing to do.

I don't think using the `Rstart` struct is a good option, as `R_SetParams()` is really something that is intended to be called once, and is fairly finicky to get right in the first place.

## @DavisVaughan at 2024-02-13T18:25:24Z

Oops, just kidding, I think I have a bug in `libr::has::R_Interactive()`. Let me fix that first.

## @dfalbel at 2024-02-14T14:07:51Z

Thank you! It's great that it works. I can see why it might not be that safe to switch `R_Interactive` within a single R session. I don't have a strong argument for doing that, besides that it's useful for the connections pane as `r_tasks` might be executing user's code and calling `readline()` in that scenario crashes the R session. 
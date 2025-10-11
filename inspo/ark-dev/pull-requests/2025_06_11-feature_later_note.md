# Add a note about polled events and the later package

> <https://github.com/posit-dev/ark/pull/836>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Documenting that switching from 200ms to 50ms helped with https://github.com/posit-dev/positron/issues/7235

We did the switch in this commit https://github.com/posit-dev/ark/commit/452b84f459bf8777da3f03c1c6cd2ce0aec71fb2

## @lionel- at 2025-06-12T08:32:05Z

hmm if I set it to 1250ms or even disable polled events entirely I still get a short time.

## @DavisVaughan at 2025-06-12T12:34:25Z

@lionel- I think I know why. Winston added `later::run_now()` to his original example, which messes things up if you're trying to rely on the `R_InputHandlers` being flushed. Try this

```r
# Put these in a block to ensure they're run without delay in between.
{
  later::later(\() { finish <<- Sys.time(); cat("Finished!")}, 0)
  start <- Sys.time()
}

# Wait for a moment after running block above.
# Then run:
finish - start
```

This respects the 1250ms delay, and gets faster with the 50ms delay.

(As he noted, we don't see the `cat("Finished!")` output in this case)

## @DavisVaughan at 2025-06-12T12:37:51Z

It's definitely tied to this delay, we run the input handlers here
https://github.com/posit-dev/ark/blob/3d42829c911c27f2ace0565378cd881e434ac8c6/crates/ark/src/sys/unix/interface.rs#L100

Which is called from `process_idle_events()`
https://github.com/posit-dev/ark/blob/3d42829c911c27f2ace0565378cd881e434ac8c6/crates/ark/src/interface.rs#L1893-L1902

Which is what we do on each 50ms tick
https://github.com/posit-dev/ark/blob/3d42829c911c27f2ace0565378cd881e434ac8c6/crates/ark/src/interface.rs#L924

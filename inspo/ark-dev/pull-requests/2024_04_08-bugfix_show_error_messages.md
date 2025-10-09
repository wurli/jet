# Don't set `show.error.messages` to `FALSE`

> <https://github.com/posit-dev/ark/pull/303>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/2694
Closes #384.
Closes #387.

In https://github.com/posit-dev/amalthea/pull/21 we have changed the `show.error.messages` global option to `FALSE` to prevent the internal error handler of R from displaying the message after we've captured it and emitted it via Jupyter. A big downside is that this has the side effect of changing the default verbosity of `base::try()`.

To fix this, I changed our global handlers to invoke the `"abort"` restart. This causes a silent longjump to top level that prevents any further error handlers from bein called, including the internal handler of R.

I've played with this a bit, including in the `browser()`, and this seems to work well (errors are emitted via Jupyter and no duplicate message is emitted as stderr stream).

## @DavisVaughan at 2024-04-09T14:20:59Z

@lionel- I'm not sure we can do this as is, it completely eats the traceback shown by `traceback()`

<img width="372" alt="Screenshot 2024-04-09 at 10 20 27 AM" src="https://github.com/posit-dev/amalthea/assets/19150088/9b95d333-aaf8-4007-92ad-22e3f2e8147c">


## @lionel- at 2024-06-07T11:54:41Z

@DavisVaughan Now rebased, in case we take another look at this approach. We could:

- Capture a call stack with `sys.calls()` (only difference is that it captures the full traceback instead of stopping at top-level-exec but that seems ok - we can probably tweak that later if needed, there's probably a way of figuring out the virtual call stack depth - oh another diffierence is that we'll miss some of the primitive contexts such as `.Call`, probably okay too) and set the base traceback with `SET_CDR(install(".Traceback"), traceback)`. Could be done from R if we expose SET_CDR.

- Run the error option handler in an `on.exit()` installed just before invoking the abort restart.

## @lionel- at 2024-06-11T11:42:20Z

The last commits implement the approach proposed in my last message.

## @DavisVaughan at 2024-06-11T15:41:02Z

Also tested that in the debugger errors in the console are printed as expected (not captured by global calling handler, but we do see it)

<img width="1474" alt="Screenshot 2024-06-11 at 10 50 09 AM" src="https://github.com/posit-dev/amalthea/assets/19150088/0c8ed32f-82b4-4548-8971-67156bed650c">

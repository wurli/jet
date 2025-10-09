# Set as interactive on startup, set `options(device =)` to a string

> <https://github.com/posit-dev/ark/pull/905>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/7681

My preferred alternative to https://github.com/posit-dev/ark/pull/895

In a fresh R session, `.Device` in RStudio returns `"null device"` and `getOption("device")` returns `"RStudioGD"`. i.e. R doesn't have a current device active, but it knows to use a function named `RStudioGD()` when it needs to make one (and indeed you can call `RStudioGD` at the console and you can find it yourself).

For these reasons, I do not think we should inject ourselves as the current _active_ device on startup as proposed by #895. I think we should also follow RStudio's lead and let the device be `"null device"` until a plot is actually required.

---

To solve the problem a different way, we just recognize that the main issue is that we are calling `grDevices::deviceIsInteractive()` to register ourselves as an interactive device too late - we are calling it at first plot time, but should call it at startup time. I have made that change in this PR, and in theory that should have been enough to fix the issue.

In practice there is one other issue I had to deal with. We set `options(device = <fn>)` where RStudio sets `options(device = "RStudioGD")`. This is surprisingly important. In a fresh session, `grDevices::dev.interactive(orNone = TRUE)` (used by `demo(graphics)`) will look to see if the current device is in the set of known devices provided by `deviceIsInteractive()`. However, if that device is `"null device"`, then it will consult `getOption("device")` and see if that _name_ is in the set provided by `deviceIsInteractive()` instead. But if you set `options(device =)` to a _function_ rather than a _name_ then you don't get to take advantage of this nice feature. I've reworked a few things to take advantage of this now. Importantly, _we now mimic RStudio exactly here_.

I've also updated docs and added tests to assert our beliefs about how these functions work on startup.

### QA Notes

Running `demo(graphics)` _in a fresh session_  with release Positron should render all the plots non-interactively:

https://github.com/user-attachments/assets/94c7504c-d912-4386-b94d-718a822d0f97

With the fix, this should now prompt you to type Enter between each plot.

https://github.com/user-attachments/assets/ea96eab4-f2ef-4a99-93a1-3d1b03c8d50b

## @lionel- at 2025-08-22T08:28:14Z

> For these reasons, I do not think we should inject ourselves as the current active device on startup as proposed by https://github.com/posit-dev/ark/pull/895. I think we should also follow RStudio's lead and let the device be "null device" until a plot is actually required.

Sorry I don't understand. What are the reasons?

## @DavisVaughan at 2025-08-22T12:49:26Z

I think Positron/ark should work like starting R at the command line or from RStudio, i.e.:
- Initially the current graphics device is `"null device"`
- `getOption("device")` contains the default device to use when a plot is required
    - In R at the command line, this is a function that invokes a Quartz device on Mac
    - In RStudio, this is `"RStudioGD"`
    - In ark, I want this to be `".ark.graphics.device"`

It seems like that approach follows standard practices rather than setting the ark graphics device as the current device on startup

## @lionel- at 2025-08-25T08:58:49Z

So  the material difference between the two approaches is that mine pushes a new device to the device ring on startup, whereas yours doesn't. I agree we should have an empty device ring on startup. (Furthermore I see that the started device in my PR is always Ark's, but we'd like the user to be able to override it e.g. to `"quartz"`.)
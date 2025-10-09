# Offset call stack by one in debugger - tweaks

> <https://github.com/posit-dev/ark/pull/799>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

@lionel- I think you're totally right with your analysis that we were off by 1, this makes much more sense now.

In fact, as I was looking closely at this change I realized that we can now rearrange this code to make it _extremely_ clear what is being popped off `calls/fns/environments` and for what reason.

- **First** value of `calls` is removed to create `top_level_call`, remaining ones are `intermediate_calls`
- **Last** value of `fns/environments/calls` are removed to create `context_{fn/environment/frame_call}`, remaining ones are `intermediate_{fns/environments/frame_calls}`

The first two commits do a little rearranging to make this very very clear. The diff isn't great, but I think if you look at `debugger_stack_info()` in isolation it should read extremely clearly now!

---

The third commit addresses a small bug I found in your original PR. `frame_name` was being used in the first branch (the fallback case) but it wasn't defined. I think the right thing to do in this fallback case is to instead try and deparse `frame_call` like we do in all other locations. Really I think the usage of `call_name()` at all here is just to try and create a slightly more user friendly context location, so if that fails for some reason we should just fall back to standard deparsing.


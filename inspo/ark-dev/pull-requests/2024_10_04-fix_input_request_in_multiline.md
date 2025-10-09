# Factor out `handle_active_request()` to correct multiline input request issue

> <https://github.com/posit-dev/ark/pull/568>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/4901

The problem was that in something like:

```r
val <- readline("prompt>")
paste0(val, "-suffix")
```

if you send that whole selection, then we'd process the `val <- readline("prompt>")` line, but then R calls `read_console()` back and we'd just immediately shove `paste0(val, "-suffix")` through as the reply to the readline request! We need to use the `prompt_info()` to recognize that an intermediate expression has put us into an `input_request` state, and handle that before we handle `pending_lines`.

The ordering of our state machine is now:
- Handle input requests
    - Falls through to event loop to wait for input reply
- Then pending lines
- Then close out active requests
    - Falls through to event loop to wait for next user input

I've accomplished this by factoring out `handle_active_request()`. This takes that big if/else branch related to the active request and gives it its own function. The logic in that if/else was actually pretty tricky, and I think it is much cleaner now. It also allows us to sneak in `handle_pending_lines()` between the input request check and closing out the active request.




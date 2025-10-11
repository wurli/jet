# Declare that we are in debug mode if any frame is marked with `RDEBUG`

> <https://github.com/posit-dev/ark/pull/346>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/2310 enough for public beta, I think

The biggest win here is that we no longer leave "debug mode" when a frame gets added to the stack that isn't marked with `RDEBUG`, which was horribly confusing because R is still in the browser but you can't click "next" using the UI, you have to go down and type it in the console to continue.

This approach technically gives an "accurate" view of the frame you are in, but it isn't particularly useful with these cases (lazy eval of args and extra try-catch frames) as you will see in the videos. I also tried the idea of _dropping_ the non-debug frames, but that ends up being just as confusing, so I think that this simpler approach of showing everything that R tells you is on the stack (even if some are not debug frames) is probably good enough for now - this way also preserves the ability to look at the stack in the bottom left and click your way up the stack of tryCatch internals if you want to.

https://github.com/posit-dev/amalthea/assets/19150088/c45937a0-ab35-444b-b848-3aa13f14cec1

https://github.com/posit-dev/amalthea/assets/19150088/fe776ae0-cb77-4a6a-92a8-566124a00b4c

https://github.com/posit-dev/amalthea/assets/19150088/0fd0f9d7-e017-412f-998f-011d2e46e367




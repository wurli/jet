# For Jupyter, `q()` should probably be overridden to error and tell you how to actually shut down the right way

> <https://github.com/posit-dev/ark/issues/554>
>
> * Author: @DavisVaughan
> * State: OPEN
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABwXx3RQ", name = "area: jupyter kernel", description = "", color = "C2E0C6")

<img width="1496" alt="Screenshot 2024-09-30 at 4 46 58 PM" src="https://github.com/user-attachments/assets/b049ea4c-1cc5-42ad-ab0a-b8a133093826">

Otherwise R just dies and Jupyter throws warnings that R isn't responding to `is_complete_request` and friends

---

We could probably send the `ask_exit` payload from the kernel -> frontend, telling the frontend to request a shutdown?
https://jupyter-client.readthedocs.io/en/stable/messaging.html#payloads-deprecated

Payloads are deprecated but there is currently no alternative, so I feel like we should just use it for now (its been deprecated for over 9 years...)

It looks like you send it as part of an `execute_reply`
https://github.com/jupyter/jupyter_console/blob/fddbc42d2e0be85feace1fe783a05e2b569fceae/jupyter_console/ptshell.py#L743-L767

## @DavisVaughan at 2024-09-30T21:18:39Z

Oh `ask_exit` is also how ipython implements the `exit` and `quit` commands! i.e. when you type that directly at the console it:
- fires this exiter https://github.com/ipython/ipython/blob/9b52ad5d0fae168679a63ecc8743541e2f127d92/IPython/core/interactiveshell.py#L1389
- which is this `ExitAutoCall` https://github.com/ipython/ipython/blob/9b52ad5d0fae168679a63ecc8743541e2f127d92/IPython/core/autocall.py#L51-L57
- which is an `ask_exit()` reply

So yea I doubt there is any way this would get removed and is probably safe to use

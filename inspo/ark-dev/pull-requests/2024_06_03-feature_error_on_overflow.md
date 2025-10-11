# Write an R error to the buffer when user input is too large

> <https://github.com/posit-dev/ark/pull/377>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Related to https://github.com/posit-dev/positron/issues/2675, I would not say it addresses that issue, but it does prevent R from crashing and returns an informative and actionable error message now, which is our target behavior to fix for beta

It seemed like writing a `stop()` call to the buffer instead is the most reasonable way to prevent a weird state. Is there anything huge I'm missing here?

<img width="705" alt="Screenshot 2024-06-03 at 2 03 12 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/f2b22252-df1c-41c6-8538-63cf9c979458">



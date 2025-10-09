# Typically remove completions starting with `.`

> <https://github.com/posit-dev/ark/pull/108>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/380
Addresses https://github.com/posit-dev/positron/issues/1352 enough to close it, but does not fully address this comment which i will re-make into its own Quarto specific issue https://github.com/posit-dev/positron/issues/1352#issuecomment-1743599276

This PR is the return of https://github.com/posit-dev/positron/commit/37eed88dd6ea649142f8e543f3ecff088b25464e, which has a lot of formatting changes but basically boils down to the small change I made here. That commit was reverted just a few hours later here https://github.com/posit-dev/amalthea/commit/e7e229128aaa511492a337a46f6e2f9d5be55199 with no context about why. I asked @kevinushey about this and he couldn't remember why he reverted this. He even mentioned in https://github.com/posit-dev/positron/issues/380#issuecomment-1499469023 that he added this "filter out `.`" behavior.

In theory we could try and keep `.` prefixed completion items from _ever_ entering the completion vector, but man that seemed like a lot of work vs this small focused filter.

Pulling from the original issues, we now have:

https://github.com/posit-dev/amalthea/assets/19150088/9eeca9cc-c6f0-4a93-9314-18c3b69255b0

https://github.com/posit-dev/amalthea/assets/19150088/11f86c7a-07f2-4d5b-bc65-9a432556290b


https://github.com/posit-dev/amalthea/assets/19150088/fb26dde8-0f18-4bc9-887f-f326e06db98b





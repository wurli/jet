# Quote before calling `describeCall()`

> <https://github.com/posit-dev/ark/pull/389>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/3446
Also addresses a similar bug in the Variables pane itself

- `load_all()` in vctrs
- `debugonce(vec_slice)`
- `vec_slice(1:5, 1)`

Previously, would evaluate the `error_call` preemptively, oops

<img width="945" alt="Screenshot 2024-06-10 at 10 42 19 AM" src="https://github.com/posit-dev/amalthea/assets/19150088/f61a6674-2fea-4360-a693-400dfa37d812">

Now shows the call's expression and doesn't evaluate it

<img width="914" alt="Screenshot 2024-06-10 at 10 46 32 AM" src="https://github.com/posit-dev/amalthea/assets/19150088/6dfaa2b5-a561-4d0f-8694-b5db748ab87c">

Similar issue with local calls that aren't promises. Before (this errored and fell back to `<call>`):

<img width="728" alt="Screenshot 2024-06-10 at 10 43 17 AM" src="https://github.com/posit-dev/amalthea/assets/19150088/1ffd2a5c-a6ed-419c-b85e-22f43c0a6b62">

After:

<img width="630" alt="Screenshot 2024-06-10 at 10 46 47 AM" src="https://github.com/posit-dev/amalthea/assets/19150088/b8bbedce-cf71-49fd-9561-0fa514bfe78f">

Found a similar issue in the Variables pane where locally assigned promises (like with `delayedAssign()`) were not displaying right. Before:


https://github.com/posit-dev/amalthea/assets/19150088/0e0f6428-d6c0-427c-922b-2a432752c939

After:


https://github.com/posit-dev/amalthea/assets/19150088/cd5307e7-0546-481a-b4b1-30a7e02a3767




# Protect against errors in `r_size()`

> <https://github.com/posit-dev/ark/pull/526>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses the crash part of https://github.com/posit-dev/positron/issues/4686 but not the underlying error. It's now turned into a log warning:

<img width="709" alt="Screenshot 2024-09-16 at 12 02 46" src="https://github.com/user-attachments/assets/19573334-9182-4813-865e-afe65c24b4a6">


## @lionel- at 2024-09-16T10:32:28Z

I'm not happy with `r_task()` panicking like that when an R error is not caught but as we progress towards our goal of eliminating it, it will become less and less costly to make it return a `Result` that the callers have to handle. cc @DavisVaughan 
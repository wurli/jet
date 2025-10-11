# `DataViewer::execution_thread()` protect from panics

> <https://github.com/posit-dev/ark/pull/24>
>
> * Author: @romainfrancois
> * State: MERGED
> * Labels:

Follow up from https://github.com/posit-dev/amalthea/pull/13#discussion_r1210693300



## @romainfrancois at 2023-06-09T15:42:26Z

ping @kevinushey in case you have views on e.g. [try blocks](https://doc.rust-lang.org/beta/unstable-book/language-features/try-blocks.html) ?

## @kevinushey at 2023-06-09T16:56:22Z

try blocks would be nice, but since they're an unstable feature I don't think we want to use it. I think emulating the same functionality with a result-returning closure is fine.

## @romainfrancois at 2023-06-12T08:13:44Z

Actually `local!` lets us do just that ...

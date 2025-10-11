# Fix various missing `r_lock!`s

> <https://github.com/posit-dev/ark/pull/102>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/1317
Addresses https://github.com/posit-dev/positron/issues/1045 (I think, see https://github.com/posit-dev/positron/issues/1045#issuecomment-1738094344)

In https://github.com/posit-dev/amalthea/pull/26/, which was never merged, I had used the tooling in that branch to automatically identify a few places where we were missing `r_lock!`s. I've extracted those out into this PR.

I believe the one related to `resolve_completion_item()` was the cause of https://github.com/posit-dev/positron/issues/1317. I can no longer reproduce the crash with this PR.

---

I feel like it may be worth circling back on the ideas in https://github.com/posit-dev/amalthea/pull/26, since they allow us to avoid some of these very hard to track down bugs. But it would be a somewhat significant amount of effort to start switching over to it. Maybe it can be done in pieces?


# Ark: Infrastructure: Reduce concurrent R evaluations to a minimum

> <https://github.com/posit-dev/ark/issues/691>
> 
> * Author: @lionel-
> * State: OPEN
> * Labels: 

Currently there are many places where we evaluate R code / access the R API through the use of `r_task()`, which hooks into R's polled events. This makes it easy to implement analytic features but has a number of downsides:

- The R event polls might be slowed down of stuck if native code running in the R session doesn't check for interrupts sufficiently often.

- While R polls at specific times such as checks for interrupts, in practice we should consider polled events to be preemptive. This is unsafe because we might accidentally call a non-rentrant function, for instance we might force a promise that causes a `loadNamespace()`, which is not reentrant, while it is already running. It is difficult to manage these risks as the project grows and many features are added, querying R all the time.

So we should strive to reduce concurrent evaluations to the bare minimum:

- [x] https://github.com/posit-dev/positron/issues/2284
- [ ] https://github.com/posit-dev/ark/issues/689
- [ ] https://github.com/posit-dev/positron/issues/2321



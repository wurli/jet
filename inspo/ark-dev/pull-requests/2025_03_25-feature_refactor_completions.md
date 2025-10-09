# Refactor completions

> <https://github.com/posit-dev/ark/pull/754>
> 
> * Author: @jennybc
> * State: MERGED
> * Labels: 

Relates to #681

The main goal is to introduce a new struct to serve as headquarters when generating completions. This gives us a home for features that we compute about the completion site, that can then be used downstream in one or multiple concrete completion sources. Two existing examples of this, each previously implemented in different ways, are:

* "parameter hints": Distinguishes whether we (think) we're completing a function invocation or a function value. See #680.
* "pipe root": If we're dealing with a pipe chain that starts with, e.g., a data frame, you might want to expose its column names as completions.

Qualities we wanted in this refactoring:

* Completion site features are lazy: computed upon first need and are not re-computed unnecessarily.
* Features and/or document don't need to be threaded through as arguments through all layers of completion sources and completion item helpers as "loose parts".
* Make the completion tooling easier to work on and debug, e.g. through more intentional logging.
* General effort to remove incidental inconsistencies across individual completion sources.

I'll make a few comments on the source as well.

(I've drafted a 2nd PR #755 that builds on this one, that proposes changes to how we manage completion items as they roll in from various sources. That's in draft form until the dust settles here.)


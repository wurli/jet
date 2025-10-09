# Add support for table summary tool

> <https://github.com/posit-dev/ark/pull/904>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

For https://github.com/posit-dev/positron/issues/8343

Implemented by Claude ✨, including the tests. Everything looks legit and I've verified the summaries match those emitted on the Python side.

Claude had access to the Python side implementation which is more complicated as it opens up a temporary data explorer comm to make requests. It decided to use the internal profile summary utils, which is simpler here.

I've also cleaned up a bit:

- Environment → Variables renames
- Use a lock in tests to avoid them confusing each other.

## @lionel- at 2025-08-22T08:26:36Z

It feels like we should have some kind of safe `struct Table` API to make this stuff less verbose, easier to use, and safer.
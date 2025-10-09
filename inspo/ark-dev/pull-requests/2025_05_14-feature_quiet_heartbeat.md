# Make heartbeat quiet in debug builds

> <https://github.com/posit-dev/ark/pull/801>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

This should make logs much easier to read by default. AFAIK the heartbeat has never been helpful for debugging but it's quite verbose so it drowns other important information.

## @jennybc at 2025-05-14T14:51:13Z

This will be handy! I've been setting the `ARK_HEARTBEAT_QUIET` env var, but I tend to forget this in between bouts of ark development.
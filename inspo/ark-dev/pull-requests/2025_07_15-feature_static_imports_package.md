# Add symbols imported in packages to workspace

> <https://github.com/posit-dev/ark/pull/872>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/2252.
Addresses https://github.com/posit-dev/positron/issues/8549.
Addresses https://github.com/posit-dev/positron/issues/8550.
Progress towards https://github.com/posit-dev/positron/issues/2321.

Branched from #870. It was rather easy to implement based on the infrastructure provided in that PR.

This fixes diagnostics for imported symbols but I was still seeing some weirdness with _local_ definitions because we didn't synchronise the indexer and the diagnostics properly:

https://github.com/posit-dev/ark/blob/7175d83f46357f2ecc57d820ef4677b3122eabba/crates/ark/src/lsp/state_handlers.rs#L414-L417

This is now fixed. I've also made a change to take into account objects assigned globally. We were detecting global functions but not other kinds of objects.

I've hacked in testthat imports inside `testthat/` files. Should be good enough for now. Will fail when people edit their `testthat.R` file with additional library loading.


### QA Notes

You should now be able to open a package like ellmer and not see diagnostics. This won't be 100% proof for all packages, but I've checked with rlang and ellmer.

See also https://github.com/posit-dev/positron/issues/8549 and https://github.com/posit-dev/positron/issues/8550 for reprexes for adjacent fixes.

## @lionel- at 2025-07-17T08:23:24Z

@DavisVaughan I'm out of the day but this should be ready for review. When I come back I'll add some diagnostics tests for packages.
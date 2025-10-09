# Implement row filtering for R Data Explorer

> <https://github.com/posit-dev/ark/pull/318>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This change adds row filtering for R to the Data Explorer. Here are all > 3 carat diamonds with J color, sorted by cut.

<img width="783" alt="image" src="https://github.com/posit-dev/amalthea/assets/470418/b513555f-4e7d-4367-9564-c0a261e56286">

Most of the heavy lifting (actually doing filter comparisons) happens in R. The Rust layer simply turns the set of row filters into R objects (using existing JSON-to-R code) and passes them to R for processing. 

For performance, we cache results of the current set of filter operations (`filtered_indices`) and sort operations (`sorted_indices`). Every time either one changes, we recompute the final set of indices that represents what the Data Explorer sees -- the `view_indices`. This means we don't need to re-sort after filtering, or re-filter after sorting; the operations are independent. 

We do pay an $O(n \log n)$  cost for combining the cached sort and filter indices into the view indices. This only gets paid when sorts/filters change, though, and I think this is generally cheaper than needing to drop into R and doing a re-sort or re-filter there. 

Actual paging on sorting/filtered data is very fast since it uses precomputed row indices. No sorting/filtering is done during page/value fetch. 

Currently supported:
- search filters
- null filters
- empty filters
- comparison filters
- "between" filters
- combining filters with and/or

Currently not supported:
- set membership filters
- filtering on complex values (e.g. list columns)
- intelligent adjusting of sort/filter state when schema changes (e.g. discarding filters on removed columns)
- reporting invalid filters to the UI


## @DavisVaughan at 2024-04-24T16:07:23Z

Actually @jmcphers I did find one weird case you may want to look at
https://github.com/posit-dev/positron/issues/2872

I think parsing user supplied values is going to be pretty tough to get right. We are probably going to need some column specific metadata to be able to perform the parse correctly. i.e. for a date-time we need to know the time zone of the column to be able to parse the user input correctly. And I'd guess that 50% of the time the user will input a date-time we can't parse, so we need to be able to do something intelligent there I guess

I also imagine we are going to need a data explorer specific R side string parser for all of this, rather than relying on json roundtripping, since it is going to rely on metadata in some scenarios like dates or factors (for levels)

## @jmcphers at 2024-04-24T18:34:56Z

> I think parsing user supplied values is going to be pretty tough to get right. 

Yeah, it will be. For date ranges in particular we will eventually want some UI for picking the date so the format can be standardized. 

I added some error handling around filter application (and an accompanying test case) so that applying one that throws an error will no longer bork the view. In a follow up PR we'll want to have R tell us which filters didn't apply correctly and what errors happened while attempting to apply them.
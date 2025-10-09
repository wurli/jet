# Fix paging in data explorer

> <https://github.com/posit-dev/ark/pull/278>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This change does a number of things:

- When row labels are generated automatically, do not send them to the front end* (addresses https://github.com/posit-dev/positron/issues/2551). These auto-generated row labels can refer to only the subset of data instead of the whole thing, resulting in inaccurate labels.
- Adds basic integration tests for the data explorer that load the `mtcars` and `women` data frames into the explorer and perform a couple sample queries on each one. These tests will be expanded as we expand the functionality of the explorer.
- Adds new launch targets to the project for convenient access to debug sessions that run the Ark unit tests and Data Explorer integration tests, respectively. 

`*` Note that this results in a _new_ bug in which the front end shows zero-based indices for R data frames. I'll address this separately.


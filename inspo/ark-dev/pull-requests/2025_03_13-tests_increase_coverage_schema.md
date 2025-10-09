# Increase coverage for schema identification in the data explorer

> <https://github.com/posit-dev/ark/pull/743>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Pairs with https://github.com/posit-dev/positron/pull/6739#issuecomment-2715762427 to increase coverage of schema identification for data types in the data explorer.

We already have specific tests for summary statistics and histograms that include categorical variables too. eg

https://github.com/posit-dev/ark/blob/491b6d5cd82e7e048ecc4a3e3ad6816c976b0479/crates/ark/src/data_explorer/summary_stats.rs#L247-L259

https://github.com/posit-dev/ark/blob/491b6d5cd82e7e048ecc4a3e3ad6816c976b0479/crates/ark/src/data_explorer/histogram.rs#L534-L561


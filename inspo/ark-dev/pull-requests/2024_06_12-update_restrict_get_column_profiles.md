# Temporarily restrict get_column_profiles to only return ColumnProfile…

> <https://github.com/posit-dev/ark/pull/397>
> 
> * Author: @softwarenerd
> * State: MERGED
> * Labels: 

In order to address https://github.com/posit-dev/positron/issues/3490, this PR temporarily restricts `get_column_profiles` to only return `ColumnProfileType::NullCount`.

`ColumnProfileType::SummaryStats` will be enabled again when the UI has been reworked to more fully support column profiles.



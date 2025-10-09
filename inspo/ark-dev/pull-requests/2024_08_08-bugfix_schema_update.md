# Data Explorer: Make sure we update the data shape after a Data Update

> <https://github.com/posit-dev/ark/pull/467>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/4286

When we verify if there was a data change in R, we check if we should send a SchemaChange or a DataUpdate event. 
When the schema changes, ie, types of columns or number of columns, we would correctly update the data shape.
For data updates we would skip this update, however it's necessary because even though the schema didn't change, it's possible that the number of rows has changed and we need to make sure the front-end knows about that.


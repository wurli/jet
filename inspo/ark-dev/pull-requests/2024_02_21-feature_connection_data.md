# Use correct definition of `table` for the connections pane

> <https://github.com/posit-dev/ark/pull/248>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Targets https://github.com/posit-dev/positron/issues/2287
Linked to https://github.com/posit-dev/positron/pull/2306

This PR adds `contains_data` RPC request handling. In the meantime we also refactored the way we access object_types metadata to more closely match what RStudio does - ie, flattenting the object hirarchy tree returned by `listObjectTypes` into a list of known object types. This affects the implementation of `contains_data` as well as the `icon_request` handling.


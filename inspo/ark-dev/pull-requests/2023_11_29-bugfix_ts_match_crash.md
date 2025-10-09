# Hold TS nodes as raw data instead of external pointers

> <https://github.com/posit-dev/ark/pull/168>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/1888

Just a quick fix for now but it would be nice to clean this up later. Dereferencing these raw vectors in far away contexts feels a bit unsafe, so probably they should be a bit more structured so that the types are tagged and can be checked at the other end.

## @lionel- at 2023-11-29T14:11:39Z

See https://github.com/posit-dev/positron/issues/1888#issuecomment-1825880090 for the fix explanation.
# Updates to support the new `GetDataValues` protocol

> <https://github.com/posit-dev/ark/pull/462>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Pairs with https://github.com/posit-dev/positron/pull/4232/

This adapts the GetDataValues RPC handler to support the new RPC protocol. The main change is that it now supports different row range selections per column.

During this refactor, I also unified the way GetDataValues and ExportDataSelection gets selections of rows, so we now have a single entrypoint that knows how to deal with `view_indices` and transform selection indices from the front-end to proper indices in the R table.

Also added support for the GetRowLabels RPC method, which is now based on `row.names` instead of using the `row.names` attribute. 


# Add support for JSON-RPC method calls over frontend comm

> <https://github.com/posit-dev/ark/pull/167>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This change adds support for JSON-RPC like method calls over the frontend comm. Today, these calls work as follows:

- The client sends an `rpc_request` to ark over the frontend comm. The request has the same fields as the standard JSON-RPC message body.
- The kernel converts the parameter array to R objects. This is done using a new JSON-to-R serializer (implemented in this PR)
- The kernel invokes the method `.ps.rpc.{foo}`, where `foo` is the method specified in the RPC request.
- The result is serialized back to JSON using the existing R-to-JSON serializer
- The response is sent back over the frontend comm.

In this way, R methods can be used to fulfill JSON-RPC requests from the frontend with zero boilerplate on either side.

A downside of this zero-boilerplate approach is that it makes it difficult to discover available RPC types, or to invoke the RPCs in a type-safe way. If too many ad-hoc contracts evolve from this we could consider some approaches to decorate or declare the RPCs up front.

As a proof of concept, a single RPC has been added that sets R's console width. 

Addresses addresses https://github.com/posit-dev/positron/issues/178 and provides the R implementation of https://github.com/posit-dev/positron/issues/1860. 

Works together with https://github.com/posit-dev/positron/pull/1899 (both are needed for the feature to be complete), but PRs can be merged in any order. 


# DRAFT: Prototyping new data viewer RPC handler for R

> <https://github.com/posit-dev/ark/pull/222>
> 
> * Author: @wesm
> * State: MERGED
> * Labels: 

Closes https://github.com/posit-dev/positron/issues/2202

I will definitely need some help on this from people who actually know what they are doing =) I'm missing many aspects of how the Rust code works and lack familiarity with the R C API.

I have a very basic "it almost works" though

<img width="1601" alt="image" src="https://github.com/posit-dev/amalthea/assets/329591/a5c782cc-5009-473a-a022-860a8a603fa9">

Things that are broken / where I need help in the short term:

* Getting the column types (I'm trying to do `sapply(df, class)` from Rust and getting lost in how to invoke that properly and then interact with the result via the Rust API)
* Properly trampolining errors into the UI. For example, I have an edge case on formatting the last column (which happens to be POSIXct) but I haven't figured out how to log information that can be seen in the UI


<img width="1212" alt="image" src="https://github.com/posit-dev/amalthea/assets/329591/f99e9bde-d33a-4336-8390-fee7b20a1586">

No urgency here but would be cool to get something basic stood up!

## @wesm at 2024-01-30T13:59:34Z

It also seems like `FormattedVector` lacks support for date/time types

## @wesm at 2024-01-31T19:43:21Z

> Errors returned from your handle_rpc() function are converted to JSON-RPC errors and normally displayed in a banner on the UI side.

I rebased and rebuilt everything and now I'm not seeing the error I was having before, so we can address that when it comes up again I guess

<img width="1296" alt="image" src="https://github.com/posit-dev/amalthea/assets/329591/b36afa67-88b6-463d-80f6-5c6f17daf9e3">

I'll work on fixing up the schemas per your advice and the nested data frame columns so we can get on track toward something mergeable 

## @wesm at 2024-01-31T19:47:42Z

Nested columns seems like it might be bit of work (especially in the context of schema paging -- since the n-th column index needs to take into account the flattening of nested tables), so probably best to leave that for a dedicated PR @jthomasmock if you have thoughts
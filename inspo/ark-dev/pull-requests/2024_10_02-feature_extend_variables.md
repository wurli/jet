# Protocol for extending the variables pane

> <https://github.com/posit-dev/ark/pull/560>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

This PR  is a proposal of a mechanism that allows package authors to implement custom behavior for objects in the variables pane. 

A description of how the protocol works is available here: https://github.com/posit-dev/ark/blob/daf9348affc0f1b190928a2fb0cad5bfadaf263c/doc/variables-pane-extending.md

See also #561 for the custom inspect behavior.

## Motivation

The main motivation is to allow package authors to improve the display of objects for which the internal R data structure does not show useful information for users. For example, reticulate R obejcts are environments containing a flag and an external pointer (`x <- reticulate::py_eval("1", convert = FALSE)`):

![image](https://github.com/user-attachments/assets/309e8771-f707-415f-8f9b-cbd0d3947429)

Or torch tensors (`x = torch::torch_randn(10, 10)`) :

![image](https://github.com/user-attachments/assets/093348b5-21fc-4c53-a7c1-5109777a339e)

Or xml2 documents (`x <- xml2::read_xml("<foo> <bar> text <baz/> </bar> </foo>")`):

![image](https://github.com/user-attachments/assets/d3230a84-ae4a-480f-971b-b729bcb847be)

## How it's implemented

Package authors can implement a set of S3 like methods, each customizing a specific part of the UI.
Methods are discovered by ark when packages are loaded and lazily stored in an environment. Package authors don't need to export these methods in their packages. See [here](https://github.com/posit-dev/ark/blob/daf9348affc0f1b190928a2fb0cad5bfadaf263c/doc/variables-pane-extending.md) for the description of methods.

When responding to events from the variables pane, we try to apply the custom methods and if the method works correctly we use it's value, otherwise we try to continue using the default implementation.

Methods are expllicitly implemented for Ark, so it's expected that package authors will take extra care to make they work correctly with Positron.

## Comparison to RStudio

RStudio uses the `utils::str` S3 in a few situations to obtain the values description, see usages of ValueFromStr in eg: https://github.com/rstudio/rstudio/blob/2175496934a54e21979461a1c67eb13ad5048cf2/src/cpp/session/modules/SessionEnvironment.R#L122

There are many guardrails though, as `str` methods are commonly implemented and package authors might not be aware of such bugs when used from RStudio.






## @github-actions at 2024-10-02T13:59:42Z

All contributors have signed the CLA  ✍️ ✅<br/><sub>Posted by the ****CLA Assistant Lite bot****.</sub>

## @t-kalinowski at 2024-10-02T14:08:22Z

I have read the CLA Document and I hereby sign the CLA

## @dfalbel at 2024-10-03T11:48:11Z

I like the idea of a single ps_variable_proxy.

I think I'd still want to customize the display value though, eg for a torch_tensor, I think the display value should be something like `Float [1:10, 1:10]`, I think the closest proxy would be a length 1 character vector with that text, which would still be displayed with quotes around it:`"Float [1:10, 1:10]"`. 

I'm fine with `display type = class(x)[1]`, we could remove `display_type`.

So the proxy would replace `has_children()`, `get_children()` and `get_child_at()`.

My main concern with a single proxy implementation is performance. The way the variables pane is currenty implemented, we don't keep any state after the representations are sent to the front-end and we only get back a vector of access keys from the front-end when we want to expand a node.

Considering we implement the second option where variable_proxy() returns a list-proxy containing classed objects implementing `variable_proxy`. So suppose that A is an object that implements the `variable_proxy()` and it's children are also objects that also implement `variable_proxy()` (For instance, A could be a reticulate object representing a Python dictionary and B a dictionary contained in A).

When we click on B, so we get it's children, we'll receive a request from the front-end containing [A, B].
To compute the children of B, we'll need to:

1. `ps_variable_proxy(A)`
2. find `B` there and then
3. `ps.variable_proxy(B)`

```
Root
└── A
    ├── B
    │   ├── B1
    │   └── B2
    └── C
```

If `A` is large, building it's `variable_proxy()` can be time consuming and we'll need to rebuild it for every. neste child one expands.  

The main reason for having two separate `get_children()` and `get_child()` is that there might be an efficient way of finding `B` in `A` without building the entire list of possible children.

IMO this will be a rare situation though, except for reticulate, most use cases I can think would only want to show a nice display value. So maybe it's fine to let package authors guarantee that variable_proxy is fast enough, even if they need to implement some kind of caching, etc. Or maybe, we want to keep some state in the variables pane so subsequent requests don't need to recompute the children.

Let me know what you think!





## @lionel- at 2024-10-03T12:10:49Z

hmm if performance is an issue I think we should be able to store the proxies in our `current_bindings`? These would only be updated if the objects change across top-level calls.

## @t-kalinowski at 2024-10-03T12:16:05Z


I think we can add support for a `ps_variable_proxy`, which could be in addition to the methods proposed in the PR, as an alternative API. However, I would not want to require that the proxy approach is the only supported API.

The 'base-atomic-proxy' approach creates difficulties for objects that cannot be easily materialized as a base atomic. This is particularly problematic for objects that would be much larger in memory when materialized as a base atomic type, and for objects that have no suitable base-atomic equivalent.

For example, large sparse arrays or arrays with smaller dtypes pose challenges. It's not uncommon to have a single-digit GB-sized NumPy array with a 'float32' dtype. Materializing that as a 'base-atomic-proxy' would mean allocating a double-digit GB double array every time the variables pane is updated. Similarly, if a user has a 10 GB array of int8s, we'd be materializing a 40 GB R integer array with each variables pane update.

(We encounter these same limitations of the vctrs-proxy approach in other places, like https://github.com/rstudio/reticulate/issues/1481)

Many objects also lack a suitable base atomic equivalent. This opens up tricky questions. For instance:

1. Python AST nodes: `import ast; n = ast.Name('x')`. Would this need to be presented as a string or an R symbol?

2. Python futures: `fut = concurrent.futures.ThreadPoolExecutor().submit(fn)`. Would reticulate have to construct an R {coro} or {future} proxy?

3. Lazy-loaded arrays like https://www.bioconductor.org/packages/release/bioc/html/DelayedArray.html: How would the correct "true" dimensions of such an array be communicated without materializing a full proxy?

## @lionel- at 2024-10-03T12:45:29Z

In my mind the variable pane should never contain GBs of data and any materialised data should be truncated - to support large data we'd need a similar approach to the data viewer and then the proxy becomes a slice proxy.

I see your points regarding flexibility of data types in the context of interop. I think this aligns with Daniel's request to keep a display value extension point?

## @t-kalinowski at 2024-10-03T12:52:44Z

> I think this aligns with Daniel's request to keep a display value extension point?

I'm probably misunderstanding, but I don't think it does. A code example might help. 

With the proxy approach, what method(s) would reticulate implement to display a NumPy array? 

## @dfalbel at 2024-10-03T13:06:32Z

I think for a numpy array, reticulate would only implement `display_value` and a proxy method that returns anything that's scalar atomic, so the arrow `\/` to list children doesn't appear for it in the variables pane (equivalent to current `has_children() = FALSE`).

Storing proxies in `current_bindings` is probably possible, but the implementation will be quite tricky, because an object implementing `variable_proxy()` could be any level nested into an object that is pointed by one of the `current_bindings`.
I can take a look at what that could look like.

## @lionel- at 2024-10-03T14:18:34Z

@dfalbel Feel free to take a look but I think you've both convinced me that the proxy approach would not be a simplification.

## @dfalbel at 2024-10-03T17:04:38Z

Ok, I didn't get to it, but happy to experiment if you think it's worth it.  I set up an example package here: https://github.com/dfalbel/testVariablesPaneExtension/tree/main/R with an example that works for R6 classes and some examples of what I'd like to make it work for torch.

## @dfalbel at 2024-10-09T13:36:10Z

Thanks @lionel- 

I agree that we should have the opposite namespacing and just changed here: https://github.com/posit-dev/ark/pull/560/commits/a21dc3748cad86b1c9c589161264f16fa3962b7f

I'm happy to chat further on how to expose this API. Currently we don't necessarily need to expose `.ark_register_method()` as we already automatically find methods in packages, but it's nice for interactive debugging and for testing purposes.

## @dfalbel at 2024-10-18T15:23:07Z

As discussed, I have updated the PR with:

- Removing auto-registration in favor of only explicit method registration.
- Added an allowed list of packages, so we can test the API for some time before making it really public.

I have torch PR implementing how packages should explicitly register methods: https://github.com/mlverse/torch/pull/1200/commits/faa11e4a1364884a11afee616e01745c48923e5c




## @t-kalinowski at 2024-11-11T17:19:41Z

@lionel- @DavisVaughan Can I ask why
- automatic discovery of methods in packages was removed,
- and only packages on the explicit allow list are allowed to register methods

I’d like to implement methods not only in reticulate but also in keras and tensorflow. The recent changes before merge feel like unnecessary added complexity, inconvenience, and communication overhead, for reasons I don’t fully understand.

## @lionel- at 2024-11-14T07:49:12Z

(1) is a choice for explicitness. This API seems sufficiently specific that magic and extra convenience did not feel warranted.
(2) is about not committing to any public interface in the short term until the mlverse had time to experiment with the API.

## @t-kalinowski at 2024-11-14T17:10:58Z

That makes sense, thanks. 

## @wesm at 2024-12-03T19:26:04Z

I was reading through this since it was mentioned in https://github.com/posit-dev/positron/issues/5573. 

A side comment: I wanted to mention that in https://github.com/posit-dev/ark/blob/main/doc/variables-pane-extending.md, Positron is not mentioned at all. Ark contains a lot of code that powers features in Positron but not easy to use elsewhere. Since Ark is intended to be usable outside of Positron, it may make sense to more clearly separate out the Positron-specific code and documentation so that people don't get the wrong idea that Ark is only relevant to Positron. 
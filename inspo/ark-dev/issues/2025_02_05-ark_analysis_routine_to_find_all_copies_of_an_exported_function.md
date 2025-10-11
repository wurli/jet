# Ark: Analysis: Routine to find all copies of an exported function

> <https://github.com/posit-dev/ark/issues/690>
>
> * Author: @lionel-
> * State: OPEN
> * Labels:

When an exported function is updated in a namespace, for instance to insert breakpoints (https://github.com/posit-dev/positron/issues/1766) or update its source references, we need to update all imported copies too. This includes:

- The global search path
- Imports environments of downstream packages
- Method tables of S3 generics
- S4 and S7??

Finding the imports envs can be done dynamically by inspecting the environments of packages in `loadedNamespaces`, with some precautions to account for the possibility that an import of the same name but from another package might exist (e.g. inspecting the name/env pair).

I think a better approach would be to semi-statically analyse NAMESPACE files based on the active libpath at the time of loading. These have all the information we need (including method information, including for lazily loaded methods in recent R versions). A big advantage is that the infrastructure built for maintaining namespace knowledge can be reused for other analysis tasks in the future.

Method registration goes from downstream to upstream so the NAMESPACE of the exported method is sufficient. For imports however, the direction is reversed and we need to inspect all open namespaces. The first difficulty is that there is currently no general package onload hook that we can use, so we'll need to add that. More generally it'd be helpful to add events for any change of lexical state (load, attach, search path reorder, libpath change).

This is possibly a nicely scoped project to experiment with maintaining up-to-date state with Salsa https://salsa-rs.github.io/salsa/about_salsa.html. Salsa is a Rust framework created by the author of rust-analyzer that makes it easy to maintain accurate caches for computations based on changing inputs, and only recompute the parts of the computation graph that have been invalidated. This sort of invalidation is relevant here as, for instance, the S3 methods provided by a generic might change at runtime and even be overwritten by other packages. More generally `load_all()` might overwite a given namespace.

Of course caching this information is probably overkill at this point in time but as we scale our introspection capabilities on top of namespace information (e.g. scope info), such a caching strategy will help with performance, especially since many of our computations occur at interrupt-time and need to be as quick as possible.


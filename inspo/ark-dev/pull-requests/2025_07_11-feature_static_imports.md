# Static diagnostics for `library()` and `require()` calls

> <https://github.com/posit-dev/ark/pull/870>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/1325
Progress towards https://github.com/posit-dev/positron/issues/2321

This PR partially fixes the issue of "unknown symbols" diagnotics in fresh console sessions by moving towards _static analysis_ of `library()` and `require()` calls:

- We analyse `DESCRIPTION` (for the `Depends:` field) and `NAMESPACE` (for `export()` directives) files.

- Exported symbols are put in scope at the call site of `library()` and `require()` calls.

This takes care of most unused symbol diagnostics but not all:

- If the symbol is used before the `library()` call, it's still unkown and will still cause a diagnostic (this is expected behaviour).

- `exportPattern()` directives are not supported yet (see https://github.com/posit-dev/positron/issues/8520)

- Exported data sets are not supported yet (see https://github.com/posit-dev/positron/issues/8521)

Since this mechanism is currently limited and is a new approach, we still use the current dynamic approach as a fallback. This means the same gestures Positron users currently use to silence diagnostics (such as evaluating `library()` calls) still work as before.

This also means we are in a weird in-between state where diagnostics are not fully static, unless the session is 100% fresh. Once the limitations of the static diagnostics have been lifted, I think we should remove the dynamic fallback. The UX consequences of removing this fallback are discussed in  https://github.com/posit-dev/positron/issues/2321#issuecomment-3072885419.

Approach:

We now examine package files installed in the session's library paths.

- New `Description` and `Namespace` structs with `parse()` methods. For DESCRIPTION we implement our own DCF parser. For NAMESPACE we use a TS query for convenience, using the `TSQuery` helper implemented in https://github.com/posit-dev/ark/pull/861.

- New `Libary` and `Package` structs with `load()` methods. A library is loaded from a set of library paths, and a package is loaded from a single library path.

  The packages in a library are loaded lazily and cached in the library. For simplicity, packages are not invalidated when installed files change. In the future, once we have Salsa and the VFS infrastructure from Rust-Analyzer, we will be able to watch for changes and automatically cache updates in a simple and efficient way.

- `.libPaths()` is called at the start of the session. This is a static value that doesn't change throughout the session. When the LSP is decoupled we'll call `R` to get the lib paths and this will be static as well. If the lib paths change, the LSP must be restarted.

  Side note: I'm realising that the decoupled LSP will generally require an `R` binary in order to work well. This is similar to Rust-Analyzer requiring `cargo` to e.g. fetch metadata, so I no longer think this is a problem.

- When a `library()` or `require()` call is encountered, we get the package from the library. This causes to load if not loaded yet. We get the exports from the namespace file to put them in scope at that point in the file, and the depends field from the description file to attach other needed packages.

- The symbols exported by a package are stored in a `BTreeMap` keyed by sorted positions in the file. When we lookup whether a symbol is defined, we simply discard exports whose position is greater than the symbol. We don't need to take masking or package ordering into account as we currently only need to check for existence of the symbol, not its type.


Note that {tidyverse} and {tidymodels} don't declare packages in `Depends:`, instead they attach packages from `.onAttach()`. I've hard-coded them for now but in the longer term we need to nudge package authors towards an explicit declaration in DESCRIPTION, such as `Config/Needs/attach:`. I've opened an issue about this in https://github.com/tidyverse/tidyverse/issues/359.


### QA Notes

With:

```r
mutate(mtcars) # Not in scope

library(dplyr)

mutate(mtcars) # In scope


ggplot() # Not in scope

# Attach handled specially
library(tidyverse)

ggplot() # In scope


plan() # Not in scope

# `future` attached via `Depends`
library(furrr)

plan() # In scope
```

You should see:


<img width="347" height="503" alt="Screenshot 2025-07-15 at 14 19 50" src="https://github.com/user-attachments/assets/35712576-46d1-4b36-9606-b9f22709d3b4" />


When you evaluate one of the `library()` calls, the corresponding diagnostics about unknown symbols _before_ the library call should disappear. That would ideally not be the case, but for now we allow this as an escape hatch to work around shortcomings of the new system.

Edit: This should also work without any diagnostics (exported S4 classes and generics):

```r
library(terra)
SpatExtent
rast()
add_legend()
```

We have backend tests for these various cases.

## @lionel- at 2025-07-15T13:54:00Z

Another thing to consider in the future is transitive imports.

File A:

```r
library(ggplot2)
```

File B:

```r
source("file_a.R")
ggplot() # In scope
```

The `ggplot` symbol is imported via `source()`.

For now this will have to be worked around by evaluating the `source()` call or the relevant `library()` calls.
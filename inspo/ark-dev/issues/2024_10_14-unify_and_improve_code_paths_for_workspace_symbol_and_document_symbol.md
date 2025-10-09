# Unify and improve code paths for Workspace Symbol and Document Symbol

> <https://github.com/posit-dev/ark/issues/582>
> 
> * Author: @DavisVaughan
> * State: OPEN
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABSPDwDw", name = "enhancement", description = "New feature or request", color = "C5DEF5"), list(id = "LA_kwDOJkuGPc8AAAABwXx7Aw", name = "area: language server", description = "", color = "C2E0C6")

Connected to https://github.com/posit-dev/positron/issues/3822

See https://github.com/posit-dev/ark/pull/571 and in particular https://github.com/posit-dev/ark/pull/571#discussion_r1797344751

- We use the indexer for workspace symbols, but not for document symbols. We should try and share more infrastructure here. Note that the indexer currently produces a _flat_ list of symbols, but for outlines it is quite useful to have a _nested_ list of symbols instead.
- We should produce nested "sections", particularly in the Document Symbol response, where it shows up in the Outline view. This would allow `## second ----` to be a child of `# first ----`, and would also nest any functions / variables inside these section headers too.


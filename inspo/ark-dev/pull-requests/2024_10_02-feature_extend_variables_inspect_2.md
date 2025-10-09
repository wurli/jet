# Extending the variables pane - Inspect

> <https://github.com/posit-dev/ark/pull/561>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

This a follow up for #560. See there for more details.

This PR adds support for custom inspect behavior using the `ark_variable_get_child()` and `ark_variable_get_children()` methods. It's made in a separate PR as the large diff is confusing to review. Most of it though, is a refactor to make the recursive behavior of `resolve_object_from_path()` more explicit. 

## @dfalbel at 2024-10-17T20:11:35Z

I lacked git skills here , so I had to squash the changes in this PR. The history became completely meaningless due to conflicts when rebasing.
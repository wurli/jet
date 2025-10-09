# Refactor symbol search

> <https://github.com/posit-dev/ark/pull/597>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Progress towards https://github.com/posit-dev/positron/issues/3822.

- Errors are now consistently propagated as `Err` and only logged at the very top.

- We no longer recurse blindly into all node types except argument lists (this is the current restriction but note that we should recurse in arg lists in the future for e.g. `lapply` and `test_that`). Instead, we only recurse in handlers of specific node types. For instance, comment sections are only expected in expression lists, and thus only handled in the expression list handler.

- We no longer pass around a mut ref of the parent. Instead we pass a store of children by value and return it once done. If a handler inserts a note in the outline, it creates a new store, recurse into its children with this new store, and adds itself to the store of its caller.

  This simpler approach is easier to understand and should solve the lifetime issues encountered in https://github.com/posit-dev/ark/pull/593.

- Bunch of tests.


## @kv9898 at 2024-10-22T15:45:26Z

Thanks for the work done, @lionel- . One thing I have been struggling with in adding nested outline to this new code structure is this:

let me illustrate with this simple commented code structure:
```r
# first ####
## second ####
a <- 1
```
The tricky problem is that `first` `second` `a` are nodes belonging to the *same parent*  in the `node.walk()`, but we need to add `first` as a child of `first`, and `a` as a child of `second`. This prevents us from happily appending stuff to `store`.

Two problems arise. First, we need pass on additional information to "tell" `a` that its symbol should be added to the children of its node brother, `second`. It seems to me that this involves an additional parameter passing between `index_node`, `index_expression_list`, `index_comment` and `index_assignment`, etc. Otherwise, the `index_` argument does not know which children of children of `store` it should add the symbol to. It seems that my current PR implementation (https://github.com/posit-dev/ark/pull/593) of only passing `section_stack` between `index_node` and `index_comment` is simpler.

Second, adding element to the children of children of the last element in `store` did not seem fun. I couldn't avoid repeatedly borrowing `&mut` references, which is not allowed. I think this really is the lifetime issue.


## @lionel- at 2024-10-23T08:29:22Z

I'm thinking that only `index_expression_list()` (added in this PR) needs to keep track of the section stack and pass the currently active store to its children.

Now that all methods _own_ their store, we no longer need any ref or mut ref. So `index_expression_list()` will be able to push and pop section stores out of its stack and we only work with owned values that we move around.

Does that makes sense?

## @kv9898 at 2024-10-23T14:28:14Z

> I'm thinking that only `index_expression_list()` (added in this PR) needs to keep track of the section stack and pass the currently active store to its children.
> 
> Now that all methods _own_ their store, we no longer need any ref or mut ref. So `index_expression_list()` will be able to push and pop section stores out of its stack and we only work with owned values that we move around.
> 
> Does that makes sense?

Oh yes it does. I wrote a new pull request to this specific branch in https://github.com/posit-dev/ark/pull/606.

My approach was to use a `store-vector` to store a "flattened" version of the nested structure, which can be edited easily during comment section handling and be assembled in the end of `index_expression_list`.

Note that this process involves assigning a `level` to any symbols involved. For the non-comment symbols, I assigned them to `usize::MAX` to avoid them becoming parents.
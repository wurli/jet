# The great `tree-sitter-r` sync up

> <https://github.com/posit-dev/ark/pull/290>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Pins to https://github.com/r-lib/tree-sitter-r/commit/36f4d75791c41c14c61554f762105029bb44dfb0, will eventually just be the `next` branch of tree-sitter-r, and `main` in the medium term future.

Addresses https://github.com/posit-dev/positron/issues/1548
Addresses https://github.com/posit-dev/positron-beta/discussions/26
Addresses https://github.com/posit-dev/positron/issues/1290
Addresses https://github.com/posit-dev/positron/issues/2175
Addresses https://github.com/posit-dev/positron/issues/2543
Addresses https://github.com/posit-dev/positron/issues/1980

Main change beyond just syncing up the new tree-sitter-r node names is that we have a new `treesitter.rs` file that contains mappings from tree-sitter-r `kind()`s to our own internal `NodeType`. This is the absolute bare bones idea for adding some indirection between us and tree-sitter-r, @lionel- plans to build on (and likely rework) this in the future to add even more semantic meaning and allow us to abstract away tree sitter functions like `child_by_field_name("body")` in favor of a `body()` method on a `FunctionDefinitionNode`.

Having Rust types to `match` against is also much nicer than bare strings, which is another nice win.

I've added a decent amount of tests as I've gone along too, especially in cases where I discovered something wasn't working right after the transition, or if we are addressing an existing known issue.

I fully expect to have at least one or two bugs pop up in this transition. I've been using this branch while working on the {treesitter} R package and it's pretty stable at this point, but I've likely missed _something_.

A few fixed cases from the issues list:

<img width="562" alt="Screenshot 2024-04-02 at 3 12 33 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/793d8ef6-bb22-42e7-92f4-b985d8854669">
<img width="414" alt="Screenshot 2024-04-02 at 3 03 53 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/5fc0e1bc-7b5a-439c-8f01-52b0549b9bd1">
<img width="259" alt="Screenshot 2024-04-02 at 3 11 54 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/1fc631e2-b30b-44f5-a268-6449ea474ee1">
<img width="426" alt="Screenshot 2024-04-02 at 8 12 41 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/f0d5ac13-4863-44d4-b593-5f7b9d2063bd">



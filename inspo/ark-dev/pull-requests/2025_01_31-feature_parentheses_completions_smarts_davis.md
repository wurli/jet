# Feature/parentheses completions smarts davis

> <https://github.com/posit-dev/ark/pull/686>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Main thoughts:

- `ParameterHints` enum rather than a boolean, as it is self documenting at the call site, i.e. at call sites you see `ParameterHints::Enabled` rather than `false`

- A single "official" `parameter_hints()` function that is your entry point for this computation. You provide it a `Node` and a `Rope` and it tells you if `ParameterHints` should be enabled or not for this context. This nicely hides all the nitty gritty details from the rest of the completion engine.

- Added a few more tests and moved them to `parameter_hints.rs`, it now feels like this file is no longer "utils", and is the official home for this feature, so it now seems somewhat obvious to me for tests to live here

- Used `node_find_containing_call()`, which I've used in another place and is a nice helper for going from argument node -> call node, which is a little tricky otherwise. Simplified lots of code with this.

- Avoided all `unwrap()`s, which we want to do if at all possible, otherwise they take the whole system down.

- Removed the idea of gathering context in favor of just inlining all the "gathering" required for that specific function. The idea of gathering context was at odds with using shared functions like `node_find_containing_call()`, and I don't think we are very worried about the performance of "gather it once, use it many times" here.


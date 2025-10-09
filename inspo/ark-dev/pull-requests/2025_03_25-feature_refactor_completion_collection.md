# More completion refactoring around how completions are marshalled

> <https://github.com/posit-dev/ark/pull/755>
> 
> * Author: @jennybc
> * State: MERGED
> * Labels: 

Builds on #754 and closes #681. The second and final of piece of that refactoring effort.

This is about managing the accumulation of completion items from composite sources.

Main ideas:

* Instead of adding CompletionItems onto an ever-growing collection, manage the accumulation in a map of completion items + their ~source(s)~. *Update: we've decided to record the first-encountered source. If a completion item shows up again, we log this event, so we have a thread to pull on, if we choose to investigate this.*
* Update the map as each new completion source is consulted. Novel items are added. Pre-existing items ~get an update (addition) to their completion source~ *that are re-contributed get logged.*
* No more need for explicit deduplication.
* Keep adding / modifying the logging for future debugging happiness

## @jennybc at 2025-04-02T23:13:45Z

This PR is updated based on this morning's discussion and I'm quite happy where it ended up. It feels pretty clean and efficient to me.

I did experiment with the `HashSet` approach, but it ended up making the code more complex, caused more cloning, and entailed an expensive search. You have to implement `Hash`, `PartialEq`, etc. for `CompletionItemWithSource` and you have to create a temporary instance just for lookup. The lookup itself is also more expensive in the `HashSet` scenario. The ownership transfer of completion items feels pretty good to me now: Source completions → `HashMap` (as values) → final `Vec<CompletionItem>`.
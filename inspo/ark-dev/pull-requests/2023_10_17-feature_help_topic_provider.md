# Implement help provider for R

> <https://github.com/posit-dev/ark/pull/113>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This change implements a help provider for R that can suggest a help topic appropriate to the cursor position and, given a help topic, show help for that topic in the Help pane. 

The change is large because the implementation required adding a new channel over which the help topic request could be delivered, which in turn required some somewhat heavy-handed refactoring of how we handle Help. The good news, however, is that I was able to clean up some old TODOs and reduce the reliance on global state; we now only start the Help backend when Positron needs it, and don't need to start the HTTP help server when using a non-Positron front end. Like Environment, we now only engage the Help machinery on demand.

There are two new pieces of functionality here:
- A Help comm that sends notifications of new help content, and has a single RPC that can be used for Positron to ask for help to be shown on a given topic.
- A new LSP method, `textDocument/helpTopic`, which is used to query for a Help topic appropriate for the cursor position. 

Must not be merged before https://github.com/posit-dev/positron/pull/1593. See that PR for the front-end Help implementation.

> [!NOTE]
> The `helpTopic` implementation here is little more than a proof of concept or stub; it just looks for the nearest identifier at or preceding the cursor. This works as long as you mash <kbd>F1</kbd> directly on or near the keyword you want help on, but it also picks up local variable names and other stuff. I'll improve it in a follow up PR, and/or we can tackle it in a separate effort. The goal here is to have a proof of concept so we can unblock getting this work done for Python.

## @DavisVaughan at 2023-10-18T19:33:01Z

Would you mind going into vroom and doing `devtools::load_all()` followed by `?vroom`. It should say

```
ℹ Rendering development documentation for "vroom"
```

but for me it no longer opens an external browser showing the help page

(my exact reprex used vctrs and `?vec_sort` but i know you have vroom locally)

I see

```
[R] 2023-10-18T21:01:18.791134000Z [ark-unknown] WARN crates/ark/src/help/r_help.rs:132: Error handling Help request: Help URL '/var/folders/by/4wf2qrmn4_j93s5k5k5fzrx00000gp/T/Rtmpw0UYqk/.R/doc/html/vec_order.html' doesn't have expected prefix 'http://127.0.0.1:15295/'
[R] 
[R] Occurred at:
[R]    5: ark::help::r_help::RHelp::show_help_url
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/help/r_help.rs:213:24
```
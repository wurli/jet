# Ignore foreign info, debug, and trace logs

> <https://github.com/posit-dev/ark/pull/41>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Dependency crates (like the selectors crate) can also call `log::debug!()` and friends, and those will show up in _our_ log output, which ends up making it extremely noisy

In particular, selectors throws a massive wall of debug output to the log when you type `library(dplyr)` into the console (I think it logs the entire help page for the dplyr package), making it impossible to find anything else there. It looks kind of like this (trimmed):


> [R] 2023-06-14T23:32:18.449825000Z [ark-unknown] DEBUG /Users/pfarland/.cargo/registry/src/github.com-1ecc6299db9ec823/selectors-0.24.0/matching.rs:470: Matching complex selector h3 for ElementRef { node: NodeRef { id: NodeId(476), tree: Tree { vec: [Node { parent: None, prev_sibling: None, next_sibling: None, children: Some((NodeId(2), NodeId(3))), value: Document }, Node { parent: Some(NodeId(1)), prev_sibling: None, next_sibling: Some(NodeId(3)), children: None, value: Doctype(<!DOCTYPE html PUBLIC "" "">) }, Node { parent: Some(NodeId(1)), prev_sibling: Some(NodeId(2)), next_sibling: None, children: Some((NodeId(4), NodeId(21))), value: Element(<html>) }, Node { parent: Some(NodeId(3)), prev_sibling: None, next_sibling: Some(NodeId(21)), children: Some((NodeId(5), NodeId(20))), value: Element(<head>) }, Node { parent: Some(NodeId(4)), prev_sibling: None, next_sibling: Some(NodeId(7)), children: Some((NodeId(6), NodeId(6))), value: Element(<title>) }, Node { parent: Some(NodeId(5)), prev_sibling: None, next_sibling: None, children: None, value: Text("R: Loading/Attaching and Listing of Packages") }, Node { parent: Some(NodeId(4)), prev_sibling: Some(NodeId(5)), next_sibling: Some(NodeId(8)), children: None, value: Text("\n") }, Node { parent: Some(NodeId(4)), prev_sibling: Some(NodeId(7)), next_sibling: Some(NodeId(9)), children: None, value: ....

There is no easy way to "filter" out these log requests from dependency crates. However, I found a crate called fern which is an implementation of the `log` facade that does have a feature for setting a log level "per target".
https://docs.rs/fern/latest/fern/struct.Dispatch.html#method.level_for

The basic idea is that when you call `log::info!("hi")` there is also a secondary field called `target:` you could fill out, like `log::info!(target: "some-target", "hi")`. This seems to be fairly rare (we don't use it at all), and the default `target:` is the module location, like `"harp::environment"`. fern parses that and uses it as a way to filter on a "per target basis". I've replicated a similar idea here.

Now:
- Log requests from harp, ark, amalthea, or stdext are logged if they meet the minimal log level
- Log requests from dependency crates are logged if they meet the minimal log level AND if they are errors or warnings (no info, debug, or trace logs from these)

---

Another alternative is to raise our default log level up to `info!` so we don't see the debug or trace log output from anyone by default, but I figure we might end up back in the same spot if some dependency crate also uses `info!`, and we already use `debug!` internally so we'd have to go back and change those.


## @DavisVaughan at 2023-06-15T19:40:36Z

I've special cased harp, ark, amalthea, and stdext. I don't think I'm missing any other of our crates?
# Only show signature help `fn(<here>)` not `fn()<here>`

> <https://github.com/posit-dev/ark/pull/296>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/2253

Our problem here was that `node.find_closest_node_to_point(context.point)` finds the _closest_ node to the user's cursor, even if that node is _completely before_ the cursor. This resulted in us continuing to show signature help for `fn()   |` where our cursor is at `|`, like what is shown in the video in the original reprex.


https://github.com/posit-dev/amalthea/assets/19150088/68556b91-b4c3-4478-af26-eee0b49af49c



I think we need a full pass through `signature_help()` to completely rewrite it with our more expansive knowledge of rust and tree-sitter (like, I'm not even sure `find_closest_node_to_point` is the right thing to use here), but for now I think the best thing to do is to bail after we find the `call` node if the user's cursor `point` is not within the parentheses, i.e. only show signature help `fn(<here>)`.

I've also finally added a little testing helper to take `"fn(@x = 2)")` and turn that into a pair of `("fn(x = 2)", Point { 0, 3 })`, which is quite useful. I used it to add a few basic tests for signature-help.


# Implement selection range LSP feature

> <https://github.com/posit-dev/ark/pull/321>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Powering "expand selection" and "shrink selection" commands, which are super cool!

By default they are bound to `Ctrl + Shift + Right/Left` which are super awkward to hit. I bound them to `Cmd + Up/Down` locally with this, as I don't care much about "go to top of document".

```
    // Swap `cursorTop` and `cursorBottom` for `Expand Selection` and `Shrink Selection`
    {
        "key": "cmd+up",
        "command": "-cursorTop"
    },
    {
        "key": "cmd+up",
        "command": "editor.action.smartSelect.expand",
        "when": "editorTextFocus"
    },
    {
        "key": "cmd+down",
        "command": "-cursorBottom"
    },
    {
        "key": "cmd+down",
        "command": "editor.action.smartSelect.shrink",
        "when": "editorTextFocus"
    }
```

https://github.com/posit-dev/amalthea/assets/19150088/4ed404d1-fe16-43b1-a626-40e32084a5b9

https://github.com/posit-dev/amalthea/assets/19150088/08d6cd4d-2095-4a3f-a664-a1310fa62d66



## @DavisVaughan at 2024-04-25T15:00:16Z

There are also two settings that I didn't particularly like, so I turned these off

<img width="625" alt="Screenshot 2024-04-25 at 10 59 44 AM" src="https://github.com/posit-dev/amalthea/assets/19150088/746538b0-2bfe-4c93-aa59-b323d9e68127">


## @DavisVaughan at 2024-04-26T13:02:40Z

I actually think the last step of your "Behaviour on main" is not quite right

```
   foo(<<bar>>, baz)
=> foo(<<bar, baz>>)
=> foo<<(bar, baz)>>
=> <<foo(bar, baz)>> # i.e. this isn't what happens
```

if you have more stuff in the file than just `foo(bar, baz)`, then you won't ever get a selection for the whole function call, it skips that node, which feels like a missed opportunity

https://github.com/posit-dev/amalthea/assets/19150088/bf854968-62e9-4066-a70c-ec1bd2389f61




## @lionel- at 2024-04-26T13:29:43Z

oh you're right, so definitely not a "regression", but I do think we could make the new behaviour even more helpful.

## @DavisVaughan at 2024-04-26T14:13:33Z

@lionel- I've tweaked it so we get your ideal behavior with calls (`fn(a)`), subset (`x[a]`), and subset2 (`x[[a]]`) nodes

Eventually being able to just request `x.opening_delimiter()` after casting `x` to an `Arguments` node type is going to be so much easier

## @DavisVaughan at 2024-04-26T14:16:13Z

Selections inside function calls also got way better with this implementation. You basically had no granularity before.

Before

https://github.com/posit-dev/amalthea/assets/19150088/635a7dc1-dd35-425f-bc8b-47296d5f5873

After

https://github.com/posit-dev/amalthea/assets/19150088/99e136a3-2371-4631-acbc-bbafe7ec49c7





## @DavisVaughan at 2024-04-26T14:19:27Z

> Expand from function to function + roxygen doc (or more generally top-level object to object + doc)

Yea rust-analyzer does this with `#[test]` attributes (and probably others) which is pretty nice. It will require a bit of restructuring because we currently look straight up the parent chain and don't consider any siblings. This will require us to look at our previous siblings.

Additionally, we currently expect that each `node` contributes exactly 1 `range` to the result, but this would imply that a single `node` in the parent chain can contribute >1 ranges:
- 1 range for the node itself
- 1 range for the node + its doc comments before it

And that makes things slightly trickier too, not impossible just needs some rethinking

## @DavisVaughan at 2024-04-26T14:25:16Z

> One thing I noted is that if I leave the default whitespace setting on, sometimes the expansion doesn't make sense. That's the case inside braced if/else expressions for instance. Should we disable it by default? Can we disable by extension or only for the whole IDE?

This does kind of stink, I do think we should try disabling this for R files through the positron-r extension, since we are saying that we provide a smart select that is holistic and you don't need these extra features. (like we did here https://github.com/posit-dev/positron/pull/2173/files)

with `"editor.smartSelect.selectLeadingAndTrailingWhitespace": true,` (bad)

https://github.com/posit-dev/amalthea/assets/19150088/e26b64b4-12a7-4f60-9cab-3bc9518fe059

with `"editor.smartSelect.selectLeadingAndTrailingWhitespace": false,` (good)

https://github.com/posit-dev/amalthea/assets/19150088/f9c8eebd-60f7-41f4-a2d7-c3894d656245

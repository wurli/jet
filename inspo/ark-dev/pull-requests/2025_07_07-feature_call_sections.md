# Collect sections in calls and emit them as document symbols

> <https://github.com/posit-dev/ark/pull/867>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:


Branched from #866.
Addresses https://github.com/posit-dev/positron/issues/8402.

You can now add sections in function calls too, between arguments.

The implementation is shared with the section logic for `{}` blocks. This works a little differently than in RStudio: The sections are nested in the call and can't affect the section hierarchy outside the call. If you have:

```r
## level 2 ----

call(
  # level 1 ----
)

call({
  # level 1 ----
})
```

The "level 1" sections in the block and call do not close the top-level "level 2" section. This difference is necessary because the LSP outline is more powerful and includes syntactic elements like function definitions and assigned variables in the outline. Allowing sections to close from inside a nested element would make things complicated and I'd argue respecting the nesting of the code makes sense from a user pespective too.


### QA Notes

Sections in calls:

```r
# level 1 ----

list(
  ## foo ----
  1,
  2, ## bar ----
  3,
  4
  ## baz ----
)

## level 2 ----

list(
  # foo ----
  1,
  2, # bar ----
  3,
  4
  # baz ----
)
```

now work the same way as sections in blocks:

```r
# level 1 ----

list({
  ## foo ----
  1
  2 ## bar ----
  3
  4
  ## baz ----
})

## level 2 ----

list({
  # foo ----
  1
  2 # bar ----
  3
  4
  # baz ----
})
```

https://github.com/user-attachments/assets/34514df8-edde-48a1-a4e4-f49a2a21ad7d

## @DavisVaughan at 2025-07-21T23:53:13Z

Ok @juliasilge, @lionel- and I talked about this quite a bit and we now think we have a good mental model that describes how this should work.

@lionel- if you could find some way to encode some of the theory into docs for this, that could be helpful for us in the future when thinking about this

## RStudio is not the best reference guide

We thought we understood how RStudio works, but we actually don't, and we don't think this behavior is right. It's very odd for `# bar` to be under `## foo`

<img width="357" height="249" alt="Screenshot 2025-07-21 at 7 09 12 PM" src="https://github.com/user-attachments/assets/ac3e1e0c-987e-4604-8c93-8f18019c7696" />

<img width="432" height="275" alt="Screenshot 2025-07-21 at 7 09 02 PM" src="https://github.com/user-attachments/assets/e8c40081-2ea7-4c9e-a58d-00cdb1d3a553" />

## `#` alone are not enough

I briefly mentioned above that we should let the number of `#` fully drive the outline, but I no longer believe that based on what @lionel- is trying to convey here https://github.com/posit-dev/ark/pull/867#discussion_r2218673920

If we truly believed that, then consider this example

```r
# top level ----

class <- R6::R6Class(
  'class',
  public = list(
    initialize = function() {
      'initialize'
    },
    # middle ----
    foo = function() {
      # inner ----
      1
    },
    bar = function() {

    }
  )
)
```

The outline we'd build based on that principle would either look like this if we didn't collect any structure from the code elements themselves, which is pretty weak for a code outline...

```
- top level
- middle
- inner
```

...or like this if we did, but this feels _deeply_ broken. `# middle` should be _fully contained_ under the `class <-` part of the outline, but it "escapes" up to top level using this principle. That's not right.

```
- top level
  - class
    - initialize
-  middle
  - foo
- inner
  - bar
```

RStudio doesn't handle this particularly well, again suggesting it is not great as a reference here. It has the "escaping" problem referenced above.

<img width="434" height="408" alt="Screenshot 2025-07-21 at 7 21 21 PM" src="https://github.com/user-attachments/assets/d84af9f3-63b6-4902-925d-9f46957e57c1" />

## Defining sub-documents

So starting from first principles, here's what we've come up with. In markdown we have a strong notion of how `# H1`, `## H2`, etc interact with each other. When you add code elements into the picture, we need a little bit more terminology to understand how it should work - it's simply not something that exists in native markdown.

Imagine the whole file as the root `Document`. Within this document, `# H1` and friends work as you'd expect.

When you do one of the following, you enter a totally new nested `Document`:
- Start a `{` scope, i.e. `function() {` or `expr({`
- Enter a function call, i.e. `fn(`

Everything from `{ -> }` or `( -> )` should be treated as its own standalone md file that gets nested into the parent `Document`. The only thing this scope inherits from the parent `Document` is the _indent level_ to start at, i.e. here:

```r
# top level ----

list(
  # section ----
  fn()
)

# top level 2 ----
```



- The file starts a `Document`, the indent level starts at 0
- `# top level ---` goes in the outline at indent level 0 and starts a scope where the indent level is 1
- `list(` starts a new `Document` inheriting indent level 1
- `# section ---` goes in the outline at indent level 1 and starts a scope where the indent level is 2
- At the `)`, the `# section ---` scope closes and we drop to indent level 1, also the inner `Document` ends
- At `# top level 2 ---`, the `# top level ---` scope closes and we drop to indent level 0, the `# top level 2 ---` goes in the outline at indent level 0, and starts a scope at indent level 1

<img width="399" height="220" alt="Screenshot 2025-07-21 at 7 37 53 PM" src="https://github.com/user-attachments/assets/0fe4f86d-1f06-463a-a6f3-ada225a45610" />

That reasoning allows the R6 example to make sense as well

<img width="273" height="178" alt="Screenshot 2025-07-21 at 7 39 40 PM" src="https://github.com/user-attachments/assets/af8dd5df-9630-42ad-82b0-9a01dd060d45" />

Each function call or `{` you see starts a new `Document` which inherits the starting indent level from the parent, but the actual usage of `# H1` and friends is scoped to just that `Document`.

## In practice, with {targets}

Theory aside, I'm mostly convinced this will work quite well with existing targets scripts, which is really the most important thing here.

I think targets users would do one of two things:

1) They have a top level `#` section, followed by `##` sections inside their `list()` call, which makes the nesting work with RStudio. This also works out of the box with Positron with this PR.

<img width="693" height="463" alt="Screenshot 2025-07-21 at 7 44 11 PM" src="https://github.com/user-attachments/assets/1b7f2231-0d98-406c-869f-7c2d16f50385" />

<img width="687" height="372" alt="Screenshot 2025-07-21 at 7 43 56 PM" src="https://github.com/user-attachments/assets/f0dc41e8-64f0-4830-84a2-2388226098a8" />


2) They don't have a top level `#` section, and instead have `#` sections inside their `list()` call. That also works out of the box with this PR.

<img width="729" height="455" alt="Screenshot 2025-07-21 at 7 45 13 PM" src="https://github.com/user-attachments/assets/166fbdf8-133a-4e21-a370-3efa7eec93fb" />

<img width="723" height="346" alt="Screenshot 2025-07-21 at 7 45 03 PM" src="https://github.com/user-attachments/assets/d20c4059-f2a3-4b5d-ae70-582a11af44b7" />

The cool thing this PR lets you do is also drop the `##` when you have a top level section if you understand that the call to `list(` opens a new `Document` that inherits the parent nest level but otherwise functions as a standalone md file. I think this is cool!

<img width="703" height="352" alt="Screenshot 2025-07-21 at 7 44 54 PM" src="https://github.com/user-attachments/assets/1886b263-8e41-410a-bfa9-6f8e18a0c970" />



## @lionel- at 2025-07-22T07:23:19Z

Thanks for this very clear summary @DavisVaughan!

Key things:

- In Ark, each level of syntax nesting creates a new "document" with independent sections
- Ark has always been doing this without complaints from users, we're just extending the notion to calls
- RStudio actually does something similar to this in some cases, albeit in a buggy way

> The cool thing this PR lets you do is also drop the ## when you have a top level section if you understand that the call to list( opens a new Document that inherits the parent nest level but otherwise functions as a standalone md file. I think this is cool!

I'll add that this is a big ergonomic advantage: While you're deep in the code, you can add sections without having to worry about the current markdown structure. I.e. let's say we have the hypothetical implementation where headers close sections across the syntax tree. In this example I'm adding nested section and need to double check the current heading structure to avoid resetting the outline. I see I need to add a level 2 section to nest it within the level 1 header:

```r
# Level 1 ----

...

imagine_a_deep_nesting({
  ## I'm adding a level 2 section here ----
  ...
})
```

If I then change the top-level structure and forget to adjust headers in nested elements, I'll unexpectedly get the confusing reset of outline in the middle of my code:


```r
# Level 1 ----

## New Level 2 ----

### New Level 3 ----
...

imagine_a_deep_nesting({
  ## Oh no! My section is now closing level 3 ----
  ...
})
```

With the approach implemented here, where each level of syntax nesting creates a new "document", I can start a new hierarchy and have it consistently nested, no matter what actual header hierarchy is at top-level. I can copy-paste the code anywhere and the outline will still work the same.

```r
# Level 1 ----

## Level 2 ----

imagine_a_deep_nesting({
  # This section is nested in top-level headers no matter what ----
  ...
})
```

## @juliasilge at 2025-07-22T14:51:53Z

Thank you for the detailed explanation on this! ❤️

I agree that the targets use case is the most important one to make sure we get right here, based on how people are talking about using this feature. I'm a little uncomfortable straying from the RStudio behavior given that we (RStudio/Posit) basically made up this feature so if there is any norm to align with, that would be it, but I am happy to defer here as long as we get those targets users supported.

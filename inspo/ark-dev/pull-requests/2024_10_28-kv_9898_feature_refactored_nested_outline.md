# Add nested outline to refactored code

> <https://github.com/posit-dev/ark/pull/611>
> 
> * Author: @kv9898
> * State: MERGED
> * Labels: 

Should solve the majority of https://github.com/posit-dev/positron/issues/3822 except for the foldable comment sections.

Also replace `OBJECT`s in tests with `VARIABLE`s.

With code:
```r
a <- 1 # important to make sure initial non-comment symbols are handled properly
b <- 2
#first level####
###########################
#==========================
#=#=#
## second level ####
#### jump to fourth level####
### Back to third level ####
## another second level ----
some_fucntion <- function(x){
  c <- 3
  # inside first level ####
  ## inside second level####
  #### inside fourth level####
  d <- 4
  ### inside third level ####
  e <- 5
}
```

The outline with the refactored code now looks like:
![image](https://github.com/user-attachments/assets/4b5b4a5a-83cd-43db-a68f-ccd0391d66ed)




## @kv9898 at 2024-10-28T15:45:31Z

Moved to here as https://github.com/posit-dev/ark/pull/606 was accidentally closed.

## @lionel- at 2024-10-29T17:10:31Z

Hi @kv9898. Could you take a look at the changes I've made? I kept your general outline of the algorithm but I've simplified the way we manage the section stack. I've also added a test using `insta` snapshots. It was getting tedious to define the expected outputs 😄 

## @kv9898 at 2024-10-29T18:13:01Z

> Hi @kv9898. Could you take a look at the changes I've made? I kept your general outline of the algorithm but I've simplified the way we manage the section stack. I've also added a test using `insta` snapshots. It was getting tedious to define the expected outputs 😄

The changes look great! I was surprised by the snapshot test - must have been a big effort!

I noted that two of the tests failed, but it seemed that they were unrelated to our changes, right? I also noted that when using the nightly Rust (I tried with 1.84 on Windows), the ark kernel keeps crashing after 1 second or so. I (not the programme) panicked about some potential fatal errors I wrote. Things have been good with 1.80.

As a side note, do you have any suggestions on the implementation of the folding range functionality? It seems that once we initiate the Folding Range Capability, other folding rules are overridden (i.e. we need to cover all existing folding rules to prevent a regression). I'm thinking of using the same node-walking strategy to create foldable comment sections, but I'm not sure whether I can reuse any `symbols` infrastructure as folding range is a separate API. Also, is the node-walking strategy good for other foldings (e.g. regions, brackets)?

## @lionel- at 2024-10-30T07:21:08Z

> I was surprised by the snapshot test - must have been a big effort!

Luckily very easy, I switched to using https://docs.rs/insta/latest/insta/ to generate the snapshot from the `Debug` format impls for our data structures. From now on we probably should default to using this sort of tests.

> I noted that two of the tests failed, but it seemed that they were unrelated to our changes, right?

These failures are from https://github.com/posit-dev/ark/issues/609

>  It seems that once we initiate the Folding Range Capability, other folding rules are overridden (i.e. we need to cover all existing folding rules to prevent a regression). I'm thinking of using the same node-walking strategy to create foldable comment sections, but I'm not sure whether I can reuse any symbols infrastructure as folding range is a separate API. Also, is the node-walking strategy good for other foldings (e.g. regions, brackets)?

I think node walking would be a good strategy for folding ranges too. The ranges will be nested and this nesting is consistent with the nesting of the syntax tree.

I guess there are two options:

- Either start from scratch with a new traversal of the tree. That's a bit wasteful in terms of performance but would lead to simpler code, and perhaps a bit of duplication.

- Or collect folding range from the same place we colect symbols.

Note that we'll change the representation of the syntax tree in the coming weeks to https://github.com/rust-analyzer/rowan. Don't worry about implementing new code with Tree-sitter, just be aware that at some point this code will have to be rewritten. The TS and rowan representations will coexist for some time so there won't be any rush to switch.

But I think this tells me that we can keep things simple for the folding range algorithm, i.e. have a separate traversal. Make a good set of tests for it. And then when we rewrite these to use the new representation we can think about merging them. But if you see a simple way to colect folding ranges in the symbol traversal, feel free to go for it!

In either case it's probably best to start small. Since implementing folding ranges disables the existing rules we have, maybe this could be implemented under a feature flag that's disabled by default. This would allow you to implement incrementally with simple PRs.

## @lionel- at 2024-10-30T07:57:06Z

Thanks for your work on this!
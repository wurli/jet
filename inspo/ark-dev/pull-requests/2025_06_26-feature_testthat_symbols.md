# Treat `test_that()` tests as document symbols

> <https://github.com/posit-dev/ark/pull/856>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses posit-dev/positron#1428.

`test_that()` blocks are now recorded as document symbols. This enables the following features:

- Hierarchical outline
- Breadcrumbs
- Document symbol search (`@` prefix in the command palette)

The tests are registered with "Test: " prefix, followed by the test title.

I've purposely not made them _workspace_ symbols, so you can't search for tests across files with a workspace symbol search (`#` prefix in the command palette). The markdown extension does that for markdown sections and I find it gets in the way a lot when searching for variables and functions.


### QA Notes

I've added backend side tests.

On the frontend with:

```r
test_that("foo", {
    # section ----
    bar <- function() {
        1
    }
    1
})
```

You should see `Test: foo`:

- in the outline (and be able to interact with it)
- in breadcrumbs (and be able to interact with them)
- in document symbols

<img width="347" alt="Screenshot 2025-06-26 at 16 47 52" src="https://github.com/user-attachments/assets/4d73c249-15af-4fef-aa8e-011d05b8b2a8" />


https://github.com/user-attachments/assets/c2130ae9-942d-4cf4-8aaa-d1d3fc72f35d


## @lionel- at 2025-06-27T06:24:34Z

> Also just a side note: the test explorer (which is a positron-r matter) needs to gain a way to register test_that()-like functions, i.e. if a user or an R package creates a wrapper that basically quacks like test_that() and should be treated accordingly by machinery like this. Whenever that happens, we'd probably want to try to make breadcrumbs also work. https://github.com/posit-dev/positron/issues/7213

I was thinking the same! Almost implemented it as a TS query as queries would probably be the way to make it generic.

It'd be nice to share the same configurability for document symbols and the test infra.

## @lionel- at 2025-06-27T06:28:12Z

> I think it would be interesting to add a test where there are comments that basically section the tests into different blocks.

I've now expanded the test with a second section.

## @jennybc at 2025-07-01T23:56:03Z

I took this for a little test drive in gargle. I'm wondering if we might just want to stop at the top-level `test_that()` calls, in terms of document symbols. At least in this example, I feel like the lower-level symbols are a net negative and somewhat overwhelming. I haven't thought deeply about this problem ... are there cases where digging into each `test_that()` for symbols is super useful?

I guess having them in the outline view is fine, because they can be collapsed. The problem of overabundance seems to hurt more in when using the command palette to search for a symbol.

https://github.com/user-attachments/assets/0177a100-fdc2-47d9-8e3b-d7875607fe7f



## @lionel- at 2025-07-03T14:26:19Z

@jennybc I think we want to recurse to support things like sections inside `test_that()` (even if it's not good practice to have very large test blocks, you still see them in the wild).

But regarding how busy the outline currently feels I think this will be mostly solved by this: https://github.com/posit-dev/positron/issues/8330
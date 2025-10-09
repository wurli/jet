# Fix ranges of comment sections

> <https://github.com/posit-dev/ark/pull/855>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses #846.
Addresses posit-dev/positron#6137
Supersedes and closes #854.

Pair-programmed with an agent ✨.

The first commit refactors things to avoid the use of explicit stacks. The second commit is the fix.

Both linked issues stem from section ranges not extending to the section end. The section now ends at either the next non-nested section, or at the closing delimiter of the enclosing node (when the section is nested in a function for instance

The section end needs to be on the previous line. In principle range ends are exclusive, but in practice we need to go back a line for vscode breadcrumbs coverages to behave properly.


### QA Notes

The fixes in this PR are tested on the backend side. On the frontend side you should see the following when testing:

With: 

```r
# https://github.com/posit-dev/ark/issues/846

{
    # level 1 ####
    1
    # another level 1 ####
    2
}

# Level ----

foo <- function() {
    1
    2
}
```

The breadcrumbs coverage (revealed when a breadcrumb is selected) should be consistent with the sections. Nested symbols such as function definitions should work as well.

https://github.com/user-attachments/assets/e0bffd43-e9d6-48b3-88a6-682e21dfdc03


With:

```r
# https://github.com/posit-dev/positron/issues/6137

test_fun <- function(...) {












    return(invisible())
  }





  #### Section Title ####
  test_fun2 <- function(...) {







    fun3 <- function() {











    }







    # long function
    return(invisible())
  }
```

The breadcrumb sticky scrolls should incorporate sections. Nested breadcrumb should still work:

https://github.com/user-attachments/assets/8f5f35a9-b955-4a13-b3cf-5afd97af6bce




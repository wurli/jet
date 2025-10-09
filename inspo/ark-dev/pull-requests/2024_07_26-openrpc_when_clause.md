# Add new `.ps.ui.evaluateWhenClause` OpenRPC method

> <https://github.com/posit-dev/ark/pull/449>
> 
> * Author: @juliasilge
> * State: MERGED
> * Labels: 

I was thinking about how to address https://github.com/posit-dev/positron/issues/2697 and have a proposed approach in this PR plus one I will open next in Positron. I looked at how various extensions (like the git extension, etc) check for whether we are in a git repo in the workspace and all that checking is typically based on context keys. In an extension specifically, you can check those keys with a [when clause](https://code.visualstudio.com/api/references/when-clause-contexts) and I realized that approach (i.e. a string) could work well as an OpenRPC method and would be flexible for other context keys we need to check from runtimes.

This PR does not yet actually check for git repos for #2697 but I wanted to get input on the approach before going much further. To see the behavior, you can check out code such as:

```r
.ps.ui.evaluateWhenClause("isLinux || isWindows")
.ps.ui.evaluateWhenClause("isMac")
.ps.ui.evaluateWhenClause("gitOpenRepositoryCount >= 1")
```

## @juliasilge at 2024-07-28T22:54:07Z

Now with 6914a61258b9edbef44ab841cddecfa5b71eaa3d, the behavior for `.rs.api.executeCommand("vcsRefresh")` has changed to address https://github.com/posit-dev/positron/issues/2697:

- If there is no repo, it is a silent no-op
- If there is a repo, the UI will refresh for the git repo state

## @DavisVaughan at 2024-07-29T14:42:45Z

A note that an easy way to test this with a git repo is to set the workspace setting:

```
"git.enabled": false
```

Then running `devtools::test()` gives me the expected popup mentioned in https://github.com/posit-dev/positron/issues/2697.

---

Another note that I want to put somewhere is that I _used_ to have an issue with this in tree-sitter-r, which had `.git` at the top level, but had an R package in a subdirectory at `bindings/r/`. I typically like to open just `bindings/r/` in its own Positron instance, but that would trigger the above issues.

I since learned that when you open a subdirectory like `bindings/r/`, you can use the Source Control panel to _point git at a parent repository_ that the panel can track instead, and then everything magically works. I can't reproduce it now, but the Source Control panel had already found the parent git folder for me, I just had to click a box to say "yes please use that" and then everything worked nicely even without this fix.
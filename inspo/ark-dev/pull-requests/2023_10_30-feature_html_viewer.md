# Register viewer for HTML output

> <https://github.com/posit-dev/ark/pull/132>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This change causes Positron to begin displaying R's HTML widgets in its Viewer and Plots panes.

<img width="1416" alt="image" src="https://github.com/posit-dev/amalthea/assets/470418/f58e332e-3aeb-4806-87e2-c4bc01426d28">

This is a partial implementation intended to unblock basic scenarios; it's missing some features, the most important of which is that external files referenced in the HTML are not resolved, so libraries/etc. are not loaded. 

## @jmcphers at 2023-10-31T21:43:47Z

>  I think this one probably should work?

It doesn't because `str_view` doesn't work with raw HTML -- it outputs JSON that gets rendered by a JS library. 

```
...
script type=\"application/json\" data-for=\"htmlwidget-f247db145195d79ed72a\">{\"x\":{\"html\":\"<ul>\\n  <li><pre>d<span class='match'>e<\\/span>f<\\/pre><\\/li>\\n<\\/ul>\"},\"evals\":[],\"jsHooks\":[]}</script>
...
```

Since external libraries don't work yet, nothing gets rendered yet. 
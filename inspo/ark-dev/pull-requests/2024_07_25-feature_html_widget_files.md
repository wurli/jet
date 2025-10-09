# Send HTML widgets as Show File events to Positron

> <https://github.com/posit-dev/ark/pull/448>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This change is the R side of https://github.com/posit-dev/positron/pull/4151; see that PR for a detailed explanation of the changes. 

## @DavisVaughan at 2024-07-26T18:13:52Z

reprex is correctly going to the Viewer pane for me, but I prefer it without the IOPub Stream message. Otherwise it conflicts with reprex's actual output

<img width="637" alt="Screenshot 2024-07-26 at 2 13 02 PM" src="https://github.com/user-attachments/assets/a33ed01b-47a8-4d29-a7ef-794d9037d36b">

I think I would be in favor of removing this Stream output entirely, as it is not something that R would normally emit and seems to have a high chance of intermingling with other stdout

## @DavisVaughan at 2024-07-26T18:18:12Z

`profvis::profvis({Sys.sleep(2)})` experience still seems a bit wonky



https://github.com/user-attachments/assets/c32a5e44-b026-460f-9463-0b59e8a7fd15




## @jmcphers at 2024-07-26T21:42:47Z

Profvis isn't showing up in the Viewer because it is explicitly asking not to, via `viewer.suppress` (which creates an object with `suppress_viewer` class instead of the `htmlwidget` class;) 

https://github.com/r-lib/profvis/blob/b8233d3f5295be64aa6b5719db457a94cfd5ebf6/R/profvis.R#L224-L227

```r
    sizingPolicy = htmlwidgets::sizingPolicy(
      padding = 0,
      browser.fill = TRUE,
      viewer.suppress = TRUE,
...
```
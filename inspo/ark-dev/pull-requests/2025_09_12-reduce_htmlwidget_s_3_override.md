# Remove S3 overrides for `shiny.tag` and `shiny.tag.list`

> <https://github.com/posit-dev/ark/pull/919>
> 
> * Author: @juliasilge
> * State: MERGED
> * Labels: 

This PR addresses:

- https://github.com/posit-dev/positron/issues/6708
- https://github.com/posit-dev/positron/issues/5338

The specific set of S3 overrides were brought over from the RStudio notebook implementation, but I think we're finding that while rendering `shiny.tag` and `shiny.tag.list` may have made sense in a notebook context, it is turning out somewhat unexpected for most use cases in a Console/Plots/Viewer situation.

It is possible that there are some R packages that render widgets with these classes, but I spent a bit of time looking through what's out there, and I didn't find anything very compelling for this treatment:
https://github.com/search?q=%22shiny.tag.list%22&type=code

With this change, what we see is that...

- kableExtra renders without error, even after loading htmlwidgets

<img width="1548" height="1104" alt="Screenshot 2025-09-11 at 7 02 51 PM" src="https://github.com/user-attachments/assets/ba9fa2ed-414d-4dbc-9363-359c75a4ddfe" />

- a `shiny.tag` prints in the console as expected, even after loading devtools

<img width="1346" height="990" alt="Screenshot 2025-09-11 at 7 04 07 PM" src="https://github.com/user-attachments/assets/0303d004-b58a-488f-8134-affdba9131c5" />

- htmlwidgets that are plots continue to render in the Plots pane

<img width="1346" height="990" alt="Screenshot 2025-09-11 at 7 05 55 PM" src="https://github.com/user-attachments/assets/7f35762d-1b9d-48ea-870e-dc6bbc7f0227" />

- all htmlwidgets I checked are behaving as expected

<img width="1346" height="990" alt="Screenshot 2025-09-11 at 7 06 27 PM" src="https://github.com/user-attachments/assets/225b8987-2f6b-414b-bf4c-c62a0ccb08fc" />


We haven't saved this file since we turned air on this repo, so I just checked in the unrelated air formatting changes while I was here.


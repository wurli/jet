# Add support for R HTML widgets

> <https://github.com/posit-dev/ark/pull/146>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This change adds support for R HTML Widgets to ARK:

<img width="669" alt="image" src="https://github.com/posit-dev/amalthea/assets/470418/b99ea7f2-347a-42d0-805e-0a2f1e53ab7e">

At a top level, the approach is as follows:

- We install an S3 method for `print.htmlwidget` that invokes our HTML widget handler. This is surprisingly difficult to do without creating user-facing warnings and errors; I've lifted and adapted some code from RStudio's notebook interface that does similar work.
- When an HTML widget is printed, we serialize its tags and dependencies to JSON. This uses a new R-to-JSON serializer, which we will probably use soon in other contexts.
- The data is sent to Positron with the special MIME type `application/vnd.r.htmlwidget`. Over on the Positron side (separate PR), a custom notebook output renderer picks up this data and renders it in the UI.

Needs https://github.com/posit-dev/positron/pull/1822 on the front end, but neither PR depends on the other, and they can be merged in any order.


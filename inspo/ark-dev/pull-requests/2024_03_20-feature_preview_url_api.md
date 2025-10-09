# Show Shiny applications (etc) in the Viewer pane

> <https://github.com/posit-dev/ark/pull/273>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This change makes it possible for local web content (such as from Shiny) to be automatically shown in Positron's Viewer pane.  It picks up a change to the UI comm, and adds some logic that was formerly a TODO around how to handle external URLs. 

Addresses https://github.com/posit-dev/positron/issues/2090.

Companion for https://github.com/posit-dev/positron/pull/2502, but the PRs can be merged in any order. 

## @juliasilge at 2024-03-20T22:32:23Z

So do we expect plumber not to get automatically picked up, as opposed to Shiny? I can't remember from when you discussed this earlier in the week. I had to manually choose the Viewer pane after clicking the URL:

![vetiver](https://github.com/posit-dev/amalthea/assets/12505835/734c9020-edba-451a-b2ea-c21a43336fb3)


## @jmcphers at 2024-03-20T23:05:44Z

> So do we expect plumber not to get automatically picked up, as opposed to Shiny?

Plumber doesn't invoke `options$browser`, which is all that's currently hooked up. 

However, I looked into it and it was pretty easy to tell Plumber to open the Swagger docs in the Viewer pane, so I hooked that up. https://github.com/posit-dev/amalthea/pull/273/commits/e7a039f43ff5266dc351797fa27e5f018d21db5a
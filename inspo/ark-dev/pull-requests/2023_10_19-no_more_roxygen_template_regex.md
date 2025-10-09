# Improve snippets provided from ark

> <https://github.com/posit-dev/ark/pull/124>
> 
> * Author: @juliasilge
> * State: MERGED
> * Labels: 

This PR makes two changes:

- Addresses posit-dev/positron#1607 by fixing the not-quite-right roxygen snippets. As I outlined in that issue, the YAML file in the roxygen2 package changed about 10 months ago and this code was never updated to reflect those changes.
- Remove duplicate snippets that were also added in posit-dev/positron#1474. It will be easier to maintain those snippets as part of the R extension, not written here in Rust.

## @juliasilge at 2023-10-19T19:24:12Z

With these changes, the roxygen snippets behave correctly now:


https://github.com/posit-dev/amalthea/assets/12505835/90643d7f-e52f-4c6e-9d8c-45ede2c5645b



## @juliasilge at 2023-10-19T19:35:06Z

We may want to change what we treat as `body` vs. `description` for these, as we are surfacing the `${1:blahblahblah)` syntax as the snippet description.

Current snippet from ark:

![Screenshot 2023-10-19 at 1 29 27 PM](https://github.com/posit-dev/amalthea/assets/12505835/0ccf1134-6802-4156-9602-add52f5c6da0)

"Normal" vscode snippet treatment: 

![Screenshot 2023-10-19 at 1 29 56 PM](https://github.com/posit-dev/amalthea/assets/12505835/55e6353f-6939-4f9f-b22c-e6a8ff8c7471)


## @juliasilge at 2023-10-19T19:42:16Z

After 30046b9b9c7892bb80710fe2dd8029eba731e66f the UI now looks like this, which I think is better and more consistent with "regular" extension snippets:

![Screenshot 2023-10-19 at 1 40 47 PM](https://github.com/posit-dev/amalthea/assets/12505835/cd092b65-4b2b-44ae-b775-5d39c9cce272)


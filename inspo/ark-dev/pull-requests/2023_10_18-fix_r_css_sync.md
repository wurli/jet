# Pseudo-sync `R.css` with upstream R

> <https://github.com/posit-dev/ark/pull/119>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/1577

I took a look at the current version of https://github.com/wch/r-source/blob/trunk/doc/html/R.css

It seems like we are missing 3 things:
- `div.vignettes a:hover` which has been there for at least 9 years and I don't think we care about
- `tr` which is new for R 4.3.1
- `span.rlang` which is new for R 4.3.1

As mentioned in https://github.com/posit-dev/positron/issues/1577, `tr` and `span.rlang` used to be hardcoded in the HTML output but last year this was moved to `R.css` (which is good!). Unfortunately for us it meant that the `vertical-align: top` wasn't automatically being applied anymore so we ended up with the arg alignment you see in my original issue in R 4.3.1.

Here is the R commit: https://github.com/wch/r-source/commit/1f5049626b84201468bb87bd5ac3eff95ee9bd34

This PR brings `tr` and `span.rlang` over to our `R.css`:
- `tr` is copied over directly, so now we get top alignment again, yay!
- `span.rlang` is for `\\R` in the `.Rd`, which is for the name of the R language. I don't think we want any additional styling here over what our `body` rules already specify (for font and color) so I left this empty. It still always gets bolded by the HTML that gets inserted.

For example, in `?NA` you see this:
https://github.com/wch/r-source/blob/0b9e72b5c7449bdd5ebb791c19bae07c4efe71f6/src/library/base/man/NA.Rd#L26C48-L26C48

which renders as this, which looks fine still:

<img width="126" alt="Screenshot 2023-10-18 at 1 36 46 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/e943e7cc-47c7-42d0-bcb4-514430877bea">

I checked and bringing over `tr` does fix the R 4.3.1 issue:

<img width="699" alt="Screenshot 2023-10-18 at 1 38 00 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/7ecc0e42-5365-4557-847f-64196d7549a7">

It is perfectly fine in R < 4.3.1 too, as our rule just gets overridden by the hard coded CSS

<img width="452" alt="Screenshot 2023-10-18 at 1 39 12 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/9701a634-b2db-4dcf-861f-14233b9a27ea">

VS in R 4.3.1

<img width="455" alt="Screenshot 2023-10-18 at 1 39 41 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/34c8b3ac-d0ad-4578-88d3-2bac63db430f">

 


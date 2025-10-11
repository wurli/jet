# DataViewer handling data frames with no names

> <https://github.com/posit-dev/ark/issues/12>
>
> * Author: @romainfrancois
> * State: CLOSED
> * Labels:

Follow up to https://github.com/rstudio/positron/issues/594

```r
x <- vctrs::new_data_frame(list(1, 2))
```

creates a data frame with no names. The environment pane handles them as for lists:

<img width="668" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/b585a146-9e2d-4496-9ec1-a24af6bdafe5">

But they can't be `View()`ed, we get a `bail!()`:

<img width="1035" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/68363ec2-a0db-431e-843e-6ee552187eed">


## @romainfrancois at 2023-05-29T10:12:52Z

Replaced by https://github.com/rstudio/positron/issues/640

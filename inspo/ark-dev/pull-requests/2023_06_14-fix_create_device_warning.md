# Catch warnings in `.ps.graphics.createDevice()`

> <https://github.com/posit-dev/ark/pull/34>
> 
> * Author: @romainfrancois
> * State: MERGED
> * Labels: 

addresses https://github.com/rstudio/positron/issues/722

## @romainfrancois at 2023-06-14T13:20:00Z

Alternatively this could check the `cairo` capability before calling `grDevices::png` here or elsewhere ?
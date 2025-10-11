# Live updates and sorting for R data explorer

> <https://github.com/posit-dev/ark/pull/287>
>
> * Author: @jmcphers
> * State: MERGED
> * Labels:

This change adds live updates and sorting for the data explorer backend for R.

Addresses https://github.com/posit-dev/positron/issues/2159 (sorting portion)
Addresses https://github.com/posit-dev/positron/issues/2386
Addresses https://github.com/posit-dev/positron/issues/2333

Most of this is accomplished by having the data explorer's backend maintain knowledge of the object it's looking at (in the form of a name/environment binding), and the current sorting state (in the form of a set of sorting keys and sorted row indices).

Includes integration tests for the new functionality.

## @jmcphers at 2024-04-02T23:24:22Z

@lionel- thanks for the detailed review! I think I've addressed all your comments, LMK if anything still looks off.

## @DavisVaughan at 2024-04-02T23:52:45Z

I'll also take a look tomorrow morning

## @DavisVaughan at 2024-04-03T13:43:51Z

I was expecting this to live update, it does in RStudio, am I doing it wrong?

```r
library(nycflights13)
x <- flights
View(x)
x <- data.frame(a = 1:10)

```

https://github.com/posit-dev/amalthea/assets/19150088/662354c9-e393-4c63-857d-f533b94a1560

Oh, the env isn't passed through in the `View()` case. I think that will be important to do. I know that's the primary way my wife pulls up the data viewer in rstudio.

```rust
#[harp::register]
pub unsafe extern "C" fn ps_view_data_frame(x: SEXP, title: SEXP) -> anyhow::Result<SEXP> {
    let x = RObject::new(x);

    let title = RObject::new(title);
    let title = unwrap!(String::try_from(title), Err(_) => "".to_string());

    let main = RMain::get();
    let comm_manager_tx = main.get_comm_manager_tx().clone();

    RDataExplorer::start(title, x, None, comm_manager_tx)?;

    Ok(R_NilValue)
}
```

## @jmcphers at 2024-04-03T22:02:55Z

> Oh, the env isn't passed through in the View() case. I think that will be important to do. I know that's the primary way my wife pulls up the data viewer in rstudio.

@DavisVaughan Not hard to do! Implemented now.

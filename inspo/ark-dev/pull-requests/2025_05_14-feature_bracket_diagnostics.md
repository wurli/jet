# Include `[` and `[[` in call argument diagnostic disabling

> <https://github.com/posit-dev/ark/pull/803>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/5271
Addresses part of https://github.com/posit-dev/positron/issues/3749 (not the pipe part)

```r
# We currently disable diagnostics anytime we are here:
fn(arg1, arg2)
  |----------|

# But not here
data[arg1, arg2]
    |----------|

# And not here
data[[arg1, arg2]]
    |------------|
```

This causes issues for data.table, who uses NSE inside `[`. We often treat function calls, subset, and subset2 pretty similarly (in the formatter they are treated almost identically). For now it seems like the right thing to do to make data.table usable in Positron is to also disable diagnostics inside subset (and subset2 for consistency).

I've done this by unifying some code paths that differed between the two and generalizing some field names.

<img width="323" alt="Screenshot 2025-05-14 at 10 25 08 AM" src="https://github.com/user-attachments/assets/28d850d6-2cba-4ddd-8e79-eaa56d6a7423" />
<img width="453" alt="Screenshot 2025-05-14 at 10 26 22 AM" src="https://github.com/user-attachments/assets/00fb36eb-3951-42d8-8be1-dedf6cd76309" />



# Fix navigate to file on Windows

> <https://github.com/posit-dev/ark/pull/887>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/8374

I think the scheme detection was getting tripped up by windows paths like `C:\`


### QA Notes

I'll add a positron-r extension test for this.

To test, open a file called `foo.R`, navigate to another tab, then run:

```r
.rs.api.navigateToFile("foo.R")
```


## @DavisVaughan at 2025-07-30T15:55:13Z

Proof!

<img width="983" height="872" alt="Screenshot 2025-07-30 at 11 54 44 AM" src="https://github.com/user-attachments/assets/914b0024-ef9b-4284-a7bf-884eb3f34dc4" />


## @lionel- at 2025-07-30T16:01:57Z

I've confirmed `View(data.frame)` still works as it should.
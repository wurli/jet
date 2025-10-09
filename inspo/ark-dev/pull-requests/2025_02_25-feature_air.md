# Format with Air

> <https://github.com/posit-dev/ark/pull/723>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Going with 4 spaces as the indent size since we have this in `.editorconfig` and that is what most of the project (Rust) uses. Some files were fully committed to using 2 spaces so the diff is every line for those files, but I checked every file and it all looks good (except for some known if/else comment issues I manually tweaked)


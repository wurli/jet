# Only traverse `public/` and `private/` module folders

> <https://github.com/posit-dev/ark/pull/63>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Followup to https://github.com/rstudio/positron/pull/821#issuecomment-1622556493

Most of the code in `modules.rs` assumes that we are only loading the `public/` and `private/` folders. However, the directory walker / importer actually ends up traversing everything, which is why `modules/data_viewer.rs` "worked" and was loaded even though it didn't exist in one of those folders.

The "watcher" only watches things in `public/` and `private/`, and our documentation only mentions these environments, so I think we should trim back the code to only look in these two folders. I ended up mimicking the code in the `watch()` method of the watcher for loading the modules.


# Temporarily pin Rust to 1.83 on Windows

> <https://github.com/posit-dev/ark/pull/683>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Related to #678 

The Windows ark binary built against Rust 1.84 doesn't work due to #678. @lionel- and I have spent a little time looking at it, and the solution isn't super obvious yet. We will have to pin to Rust 1.83 until we can figure it out.


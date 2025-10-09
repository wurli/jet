# Bump required Rust version to 1.85

> <https://github.com/posit-dev/ark/pull/762>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

This fixes some weird LSP issues I was seeing in Zed where it was using Rust 1.80 for diagnostics and was getting confused.

It also better matches our actual Rust version we build with.


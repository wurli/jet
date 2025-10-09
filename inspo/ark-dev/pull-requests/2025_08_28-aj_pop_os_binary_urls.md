# Add support for determining the right Linux binary URL for Pop!_OS

> <https://github.com/posit-dev/ark/pull/915>
> 
> * Author: @atheriel
> * State: MERGED
> * Labels: 

This commit is a simple fix that ensures we can generate the right Linux binary URLs for Pop!_OS users.

Pop OS is a fairly popular desktop distro that is binary-compatible with Ubuntu, so e.g. `jammy` binaries from Public Package Manager work out of the box.

We never had to worry about detecting this distro for Workbench because it's desktop-only, but it's nice to wire things up in Ark to benefit desktop Positron users (like me!) on this distro.


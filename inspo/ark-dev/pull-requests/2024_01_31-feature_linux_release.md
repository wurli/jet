# Add workflow to build .deb and .rpm binaries

> <https://github.com/posit-dev/ark/pull/227>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 



## @lionel- at 2024-01-31T21:35:20Z

I considered that too but thought it'd be helpful for Linux users to provide them with actual packages until their distros do. We could simplify if this turns out to be a bother. I don't mind simplifying now if you prefer.

## @jmcphers at 2024-01-31T22:00:33Z

Very nice of you! We can leave as is since you've already got it set up, hopefully not too much hassle to extract the files from these over on the consumption side. 

## @lionel- at 2024-02-01T06:07:42Z

Extracting from a .deb is just one ar followed by one tar.

## @lionel- at 2024-02-01T16:01:34Z

@jmcphers I see what you mean regarding ease of use on the consumer side. It was easier to build a zip of the built ark and not worry about special-casing linux in `install-kernel.ts`. Also the cargo tools for building these packages took quite a while to compile. So I changed this workflow to only build a zip for now.

I did experiment with caching the cargo deps so we could use that to speed up these deb and rpm releases if we wanted. Here is how it looks for our current release workflow: https://github.com/posit-dev/amalthea/compare/main...feature/cache. That only brings down the build time from 14m to 6m so I didn't bother opening a PR. IIRC on the CI side it'd bring build times from 4m to about 1m, but 4m is already very fast too.

Another thing, I noticed that we were building all release assets on every push to main even if we were not going to use them. I fixed that in https://github.com/posit-dev/amalthea/commit/cb553600b38c7e226f82cf1cd9432008e206640e.
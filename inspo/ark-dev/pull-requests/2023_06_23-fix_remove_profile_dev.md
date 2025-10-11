# Remove apparently unused `profile.dev` config

> <https://github.com/posit-dev/ark/pull/56>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

`cargo build` always throws this warning, and today I figured out why

<img width="829" alt="Screen Shot 2023-06-23 at 5 00 41 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/d06b2efa-81f5-4fd6-bfde-71cd5a082084">

There is a `profile.dev` config option that we set that is being completely ignored, because apparently they have to be set in the top level `Cargo.toml` file.

This `profile.dev` config bit was added here:
https://github.com/posit-dev/amalthea/commit/09522ad5d9603feb3a05f0979ca062dbcc9ab839

But I don't think it does anything now? It has been ignored for awhile, it seems.

@jmcphers is that correct?

## @jmcphers at 2023-06-26T17:26:52Z

Yes, I think that's a relic!

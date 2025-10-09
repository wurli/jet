# Correctly handle primitives when looking up package name

> <https://github.com/posit-dev/ark/pull/351>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/3117

We were calling `utils::packageName()` on what ended up being a `NULL` environment, and that throws an error. Our R traceback capturing machinery was capturing the _inlined_ objects in the upstream call to `.ps.completions.formalNames()` where the entire tibble with its nested list-columns were being inlined as the `object` there, which exceeded the 512mb limit allowed for a string that can be created on the Node side.

This fixes the underlying issue of calling `packageName(NULL)`, but we should probably also not inline objects into calls (i.e. add machinery to evaluate in a child env instead). (We have an issue for this, see https://github.com/posit-dev/ark/issues/695)

The video just shows that it no longer hangs

https://github.com/posit-dev/amalthea/assets/19150088/58134d27-977c-4b96-85da-a35e413d720d




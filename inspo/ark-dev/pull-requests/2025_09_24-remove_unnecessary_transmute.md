# Get rid of warnings about unnecessary transmutation

> <https://github.com/posit-dev/ark/pull/925>
> 
> * Author: @jennybc
> * State: MERGED
> * Labels: 

I'm about to do ark dev work on Windows, so I tried to make sure it was building and testing cleanly first. I discovered some warnings about unnecessary transmutation. I'm not sure if we want to handle this with changes like this PR or by turning off such warnings? I used Copilot to create these changes.

Also, I see that this file was originally created with a code generation process, but it seems like it's received various manual tweaks since then, so I assume it's fair game.

I paste the warnings below and also link to a recent CI run, where these are also showing up:

https://github.com/posit-dev/ark/actions/runs/17880174813/job/50846807788#step:8:22

```
   Compiling libr v0.1.0 (C:\Users\jenny\ark\crates\libr)
warning: unnecessary transmute
   --> crates\libr\src\sys\windows\types.rs:211:18
    |
211 |         unsafe { ::std::mem::transmute(self._bitfield_1.get(16usize, 16u8) as u32) }
    |                  ---------------------^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |                  |
    |                  help: replace this with: `u32::cast_signed`
    |
    = note: `#[warn(unnecessary_transmutes)]` on by default

warning: unnecessary transmute
   --> crates\libr\src\sys\windows\types.rs:216:28
    |
216 |             let val: u32 = ::std::mem::transmute(val);
    |                            ---------------------^^^^^
    |                            |
    |                            help: replace this with: `i32::cast_unsigned`

warning: unnecessary transmute
   --> crates\libr\src\sys\windows\types.rs:231:47
    |
231 |             let RstartVersion: u32 = unsafe { ::std::mem::transmute(RstartVersion) };
    |                                               ---------------------^^^^^^^^^^^^^^^
    |                                               |
    |                                               help: replace this with: `i32::cast_unsigned`
```

## @jennybc at 2025-09-24T05:08:54Z

Indeed the warnings are no longer there in CI: https://github.com/posit-dev/ark/actions/runs/17966840203/job/51100976919#step:8:21
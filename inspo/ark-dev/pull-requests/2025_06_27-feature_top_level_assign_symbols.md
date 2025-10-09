# Don't emit nested objects as document symbols

> <https://github.com/posit-dev/ark/pull/859>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Branched from #858
Addresses https://github.com/posit-dev/positron/issues/8330

Objects assigned in `{}` blocks are currently emitted as document symbols, which causes the ouline and document symbol search (`@` prefix in command palette) to be quite busy:

https://github.com/user-attachments/assets/00f12f0b-8ef0-4a6d-bd43-15e8a614bb1b

Here is how it looks like if we only emit top-level objects:

https://github.com/user-attachments/assets/ac81233b-7e8a-4139-abb2-8ef5dd9b4c99

This is controllable via a new `"positron.r.symbols.includeAssignmentsInBlocks` setting. By default we don't include these nested assignments.


### QA Notes

You should see the new setting in the config UI:


<img width="753" alt="Screenshot 2025-07-05 at 09 44 17" src="https://github.com/user-attachments/assets/5ec6c1fd-2916-436d-a475-4c3f36cd270a" />

It's documented that files need to be reopened (they can also be changed) or the server restarted for this setting to take effect.



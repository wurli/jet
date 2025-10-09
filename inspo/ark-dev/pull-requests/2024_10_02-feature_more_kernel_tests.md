# Test multiple expressions in console and notebook modes

> <https://github.com/posit-dev/ark/pull/559>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Branched from #558.

- Add a Notebook variant of our dummy kernel. It's tested in its own `tests/` file.

- Test multiple expressions in console and notebook modes. In notebook mode, intermediate results are not printed. In console mode, they are. This is relevant for the case of selection sent to the console in Positron, until posit-dev/positron#1326 is fixed.


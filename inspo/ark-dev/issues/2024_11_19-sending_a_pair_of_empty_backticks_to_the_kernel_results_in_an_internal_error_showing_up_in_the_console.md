# Sending a pair of empty backticks to the kernel results in an internal error showing up in the console

> <https://github.com/posit-dev/ark/issues/598>
> 
> * Author: @DavisVaughan
> * State: OPEN
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABSPDwBw", name = "bug", description = "Something isn't working", color = "E99695"), list(id = "LA_kwDOJkuGPc8AAAABwXx3RQ", name = "area: jupyter kernel", description = "", color = "C2E0C6")

![Image](https://github.com/user-attachments/assets/f1319dfc-f8bd-489f-b6c5-c189f7ab8b55)


## @DavisVaughan at 2024-11-19T21:40:29Z

Similarly, `_;` triggers this invalid pipe placeholder parse error
https://github.com/wch/r-source/blob/988774e05497bcf2cfac47bfbec59d551432e3fb/src/main/gram.y#L1755

![Image](https://github.com/user-attachments/assets/5157c7c1-a0c6-4143-add7-f96ead280c26)

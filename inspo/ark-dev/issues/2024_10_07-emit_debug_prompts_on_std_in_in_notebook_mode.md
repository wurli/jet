# Emit debug prompts on StdIn in notebook mode

> <https://github.com/posit-dev/ark/issues/572>
> 
> * Author: @lionel-
> * State: OPEN
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABwXx3RQ", name = "area: jupyter kernel", description = "", color = "C2E0C6")

The current experience is confusing:

![Image](https://github.com/user-attachments/assets/c9cc6626-30b2-4663-9aa7-b2a31c2ba933)

If these prompts were emitted on StdIn, a debug session would look more like this:

![Image](https://github.com/user-attachments/assets/40d07fcb-0007-493d-9e9a-43cbd2ee804a)


## @lionel- at 2024-10-07T09:39:53Z

Note that in Positron the stdin prompts are currently broken: https://github.com/posit-dev/positron/issues/4920
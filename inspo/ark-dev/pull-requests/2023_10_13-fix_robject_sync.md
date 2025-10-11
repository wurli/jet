# Introduce thread safe `RObject` variant and remove `Send` from `RObject`

> <https://github.com/posit-dev/ark/pull/111>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/1550

Joint work and design with @lionel-

Follow up PR will be to look into removing `Sync` and `Send` from `Binding`

## @DavisVaughan at 2023-10-16T19:58:56Z

@lionel- `Binding` seems to be deeply intertwined with the environment pane, so I think it is going to take a significant chunk of work to detangle that. Do you mind looking at what I have here to see if we can merge this?

# Ark failing to run functions in Zed

> <https://github.com/posit-dev/ark/issues/715>
> 
> * Author: @aymennasri
> * State: CLOSED
> * Labels: 

When trying to define a simple function in Zed without the REPL cell, Ark panics throwing out a bizarre error.

![Image](https://github.com/user-attachments/assets/893edac5-3354-4027-aa56-ccbeacf49abb)


## @lionel- at 2025-02-19T10:56:08Z

I think that's because Zed should check for the complete expressions / statement range at point but these features are not part of  the official Jupyter protocol. So for the time being you'll have to select code that you want to run.
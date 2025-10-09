# Can Ark replace `languageserver` in VS Code workflow?

> <https://github.com/posit-dev/ark/issues/637>
> 
> * Author: @AdaemmerP
> * State: CLOSED
> * Labels: 


Thanks for this great project! 

Does Ark already work as a replacement for `languageserver` or other components within VS Code as outlined in this workflow: https://code.visualstudio.com/docs/languages/r ? In particular, does it work in Code server? If not, how will Ark work within VS Code, apart from Jupyter Notebooks?

## @lionel- at 2024-11-25T11:55:22Z

Unfortunately this is not currently possible. But we do have plans to make the LSP independent from the Jupyter kernel.
# Eliminate built-in snippets

> <https://github.com/posit-dev/ark/issues/780>
> 
> * Author: @jennybc
> * State: CLOSED
> * Labels: 

*Interconnected with https://github.com/posit-dev/positron/issues/7234, https://github.com/posit-dev/positron/issues/3108, and #779.*

The existing built-in snippets and my proposed fate for them:

| Proposal | Prefix | Description | Body |
|----------|--------|-------------|------|
| delete | lib | Attach an R package | `library(${1:package})` |
| delete | src | Source an R file | `source("${1:file.R}")` |
| delete | ret | Return a value from a function | `return(${1:code})` |
| delete | mat | Define a matrix | `matrix(${1:data}, nrow = ${2:rows}, ncol = ${3:cols})` |
| delete | sg | Define a generic | ```setGeneric("${1:generic}", function(${2:x, ...}) {\n\tstandardGeneric("${1:generic}")\n})``` |
| delete | sm | Define a method for a generic function | ```setMethod("${1:generic}", ${2:class}, function(${2:x, ...}) {\n\t${0}\n})``` |
| delete | sc | Define a class definition | `setClass("${1:Class}", slots = c(${2:name = "type"}))` |
| keyword source | if | Conditional expression | ```if (${1:condition}) {\n\t${0}\n}``` |
| keyword source | el | Conditional expression | ```else {\n\t${0}\n}``` |
| delete | ei | Conditional expression | ```else if (${1:condition}) {\n\t${0}\n}``` |
| keyword source | fun | Function skeleton | ```${1:name} <- function(${2:variables}) {\n\t${0}\n}``` |
| keyword source | for | Define a loop | ```for (${1:variable} in ${2:vector}) {\n\t${0}\n}``` |
| keyword source | while | Define a loop | ```while (${1:condition}) {\n\t${0}\n}``` |
| delete | switch | Define a switch statement | ```switch (${1:object},\n\t${2:case} = ${3:action}\n)``` |
| delete | apply | Use the apply family | `apply(${1:array}, ${2:margin}, ${3:...})` |
| delete | lapply | Use the apply family | `lapply(${1:list}, ${2:function})` |
| delete | sapply | Use the apply family | `sapply(${1:list}, ${2:function})` |
| delete | mapply | Use the apply family | `mapply(${1:function}, ${2:...})` |
| delete | tapply | Use the apply family | `tapply(${1:vector}, ${2:index}, ${3:function})` |
| delete | vapply | Use the apply family | `vapply(${1:list}, ${2:function}, FUN.VALUE = ${3:type}, ${4:...})` |
| delete | rapply | Use the apply family | `rapply(${1:list}, ${2:function})` |
| delete | ts | Insert a datetime | `` `r paste("#", date(), "------------------------------\\n")` `` |
| shiny extension | shinyapp | Define a Shiny app | ```library(shiny)\n\nui <- fluidPage(\n  ${0}\n)\n\nserver <- function(input, output, session) {\n  \n}\n\nshinyApp(ui, server)``` |
| shiny extension | shinymod | Define a Shiny module | ```${1:name}_UI <- function(id) {\n  ns <- NS(id)\n  tagList(\n\t${0}\n  )\n}\n\n${1:name} <- function(input, output, session) {\n  \n}``` |

## @jennybc at 2025-04-18T20:21:50Z

I just discovered that the VS Code Python extension came to a similar conclusion and got rid of built-in snippets: https://github.com/microsoft/vscode-python/issues/14781. Lots of the same reasoning, as well.

There's a more recent PR re: disabling snippet-y treatment of keywords to bring the Jedi experience in line with Pylance: https://github.com/microsoft/vscode-python/pull/21194.
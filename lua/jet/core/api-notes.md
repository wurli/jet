
| fn            | statusses | action                                            |
| --------      | --        | --                                                |
| get_inactive  | inactive  | start                                             |
| get_connected | connected | get                                               |
| get_external  | external  | attach                                            |
| get_all       | all       | inactive=>start, connected=>get, external=>attach |

Use-case:

`get_all`:       Quick 'no thinking' mode. Not typically for programmatic use.
`get_connected`: Mainly programmatic use, e.g. keymaps
`get_inactive`:  Programmatic and maybe interactive too?
`get_external`:  Programmatic and maybe interactive too?


# Newly constructed document context may get the wrong node

> <https://github.com/posit-dev/ark/issues/778>
> 
> * Author: @jennybc
> * State: CLOSED
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABwXx7Aw", name = "area: language server", description = "", color = "C2E0C6")

We're teasing this buglet out of https://github.com/posit-dev/ark/issues/770

When completions are triggered on, for example, an empty line, the current point isn't inside a syntax node. Or, rather, it's only inside the enclosing 'Program' node, which is sort of a degenerate case.

`find_closest_node_to_point()` will then latch on to a nearby node in some cases when it should not. (Which then interacts poorly with our completion logic, which is what #770 about.)

<img width="744" alt="Image" src="https://github.com/user-attachments/assets/9c76437b-f92b-4320-a682-f24b9c7d23c3" />

Above, I've triggered completions on an empty line, but according to the document context, the completion node is the closing paren `)` of the line above.

We suspect this may be an edge case that hasn't been accounted for, i.e. that the current behaviour is unintentional. I'm going to see if we can change/fix this without breaking other things.


# Add basic debugger variables pane support

> <https://github.com/posit-dev/ark/pull/307>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/1764
See also https://github.com/posit-dev/positron/pull/2722

Features and restrictions:
- This PR adds basic DAP support for `Scopes` and `Variables` requests.
- You can click on any frame in the stack, and the variables with auto update
- You can define new variables using the R Console, and the variables with auto update
- There is some basic support for showing a variable _value_ in the debug variables pane.
  - Basic atomic vectors show up to 5 values
  - Bare lists and environments can expand to show their children indefinitely.
  - Classed objects just show their class name
  - Active bindings are treated smartly, and aren't evaluated
  - Promises are treated smartly, and their values are unwrapped if already evaluated. Otherwise we unwrap the expression and try to display that
- An annoying thing to note is that the environment used for Console _completions_ is still the global env. So you might see `foobar` in the debug variables pane, but you can't autocomplete it in the console

Obviously this is a separate code path from what we have in the "normal" variables pane. In theory the formatting code could be shared, but in practice the DAP protocol is different enough from the way we show things in the variables pane that it feels like it would be overengineering to try and align them perfectly right now.

We support a single `Scope` type - `Locals`. This shows up as a tab under the debug `Variables` header. Other languages show more tabs, like Rust has `Globals` and `Registers` as well, but I don't think we need those.

---

The way it generally works is:
- We get a debug request, so we compute the stack of `FrameInfo`s. Note that each `FrameInfo` now has its own unique `id` (they used to all get the thread id 😆 ). We also now retain the R frame environment for each `FrameInfo` (i.e. the corresponding `sys.frame()`)

- We also immediately associate each `FrameInfo` `id` with a unique `variables_reference` stored in a `frame_id_to_variables_reference` hash map. In addition, we associate that `variables_reference` with the R frame environment that will be used to collect the variables of that frame in a second hash map named `variables_reference_to_r_object`.

- A `Scopes` request comes through from the frontend, and the `ScopesArguments` contains a `frame_id` the frontend is requesting scopes for. We send back our 1 `Locals` scope along with the `variables_reference` i64 associated with that `frame_id` using `frame_id_to_variables_reference`.

- A `Variables` request comes through from the frontend for that `variables_reference`. We extract out the associated R object using `variables_reference_to_r_object`. We pass that on to `object_variables()` to collect the `Vec<Variable>` to send back.

  - Here's where things get interesting with nested structures. Say a frame env contains a named list as a child. When we collect that named list in `Vec<Variable>`, we also give that list its own unique `variables_reference` and insert it into the `variables_reference_to_r_object` hash map. The `Variable` for that list contains the `variables_reference`, so the frontend is able to lazily request that list's children when the user expands the drop down box for it. The fact that R objects can either come from `FrameInfo` frame environments or arbitrary children of those environments is why we have 2 hash maps to manage all this.

---

https://github.com/posit-dev/amalthea/assets/19150088/e393897e-ef35-49ba-9130-8225ea2a43cb

It is also responsive to changes made in the console:

https://github.com/posit-dev/amalthea/assets/19150088/9d3fd02c-0686-41c5-8f62-c5a86fa28ece

And nicely supports recursive structures (bare lists and environments, we can add more over time)


https://github.com/posit-dev/amalthea/assets/19150088/93b3d624-e6a2-4f11-9b0a-b2ef29bd6439





## @DavisVaughan at 2024-04-17T13:34:07Z

> a general purpose object explorer for developers, whereas the positron variable pane would remain focused for data analysis

Yea, I tend to agree. I also think things like `vctrs_rcrd` objects should be expandable (i.e. treat them like a bare list) for debugging purposes.

## @lionel- at 2024-04-17T13:39:00Z

>> a general purpose object explorer for developers, whereas the positron variable pane would remain focused for data analysis
> 
> Yea, I tend to agree. I also think things like vctrs_rcrd objects should be expandable (i.e. treat them like a bare list) for debugging purposes.

@hadley suggested this would be better suited for a `View()` method for lists/environments so you could have a big pane. Maybe we can reuse the same principle of exhaustiveness and the underlying format methods in both views though.

## @lionel- at 2024-04-17T15:34:27Z

@DavisVaughan Thinking more about our discussion on being defensive against race conditions, I'm now thinking it would be worth making all scope and variable ids unique. This way we can detect outdated requests as the ID would not point to any known object after we cleared it. The outdated requests could happen for instance if the user types `n` in the console (or if a frontend does something funny). We'd just reply an error in that case.

## @DavisVaughan at 2024-04-17T17:20:13Z

Ok I think that is straightforward. I'll add a `current_frame_info_id` variable to `RMainDap` that increments and is only reset during `stop_debug()` and use that in place of the iteration index for the id

## @DavisVaughan at 2024-04-17T18:07:02Z

@lionel- https://github.com/posit-dev/amalthea/pull/307/commits/0c007d246b5adc88819d299d6126793e4a02c43f should ensure that `variables_reference`s and `frame_id`s are completely unique _within a single debug session_. i.e. their counters are only reset on a `stop_debug()` call, _not_ between "steps". I've added comments about this in key places too.
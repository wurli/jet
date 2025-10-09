# Always display NA as unquoted in the Variables pane

> <https://github.com/posit-dev/ark/pull/432>
> 
> * Author: @yutannihilation
> * State: MERGED
> * Labels: 

This pull request addresses https://github.com/posit-dev/positron/issues/3810

Since `format()` and `format_one()` belongs to `Vector`, I think `format_with_string_options()` should also belong to `Vector` and be called there. But, I'm not sure if it's a good idea to include `FormattedVectorCharacterOptions` in the signature of all types of `Vector`. Considering https://github.com/posit-dev/positron/issues/2860 will introduce a facility to handle this kind of distinction easier, I'm not sure if this is worth, but I share this code just in case this helps.

## @yutannihilation at 2024-07-10T09:26:16Z

Thanks, then I mark this as ready.

One note is that I'm wondering if `Vector` might be better to have an associated type of `Options` instead of using `FormattedVectorCharacterOptions` for all types of vectors (e.g. a numeric vector needs an option for the number of digits?). At the moment, I don't see the necessity for it, but we might revisit here in future.

## @yutannihilation at 2024-07-11T02:01:07Z

Thanks for the review! Renamed the struct and add a test.

Also, thanks for the context. So, maybe it needs some more cleanup? I want to keep this pull request small for now (mainly because I don't know well about the implementations), but I can address in a separate pull request.

## @yutannihilation at 2024-07-12T04:36:09Z

Thanks! I inlined `format_with_options()` and added a note.
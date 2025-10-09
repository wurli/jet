# Add default argument values to signature labels

> <https://github.com/posit-dev/ark/pull/439>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/2779

It is best to read this PR 1 commit at a time. Each commit is nicely self contained.

Adds support for showing default argument values in the function signature label. This makes the signature label wayyyy more useful for R code, as it often shows you the set of possible enum values. It's also just typically useful to know the default arguments of the function you are calling.

<img width="536" alt="Screenshot 2024-07-16 at 5 20 18 PM" src="https://github.com/user-attachments/assets/79f5473b-fa76-4d74-8028-c4954600bceb">

<img width="719" alt="Screenshot 2024-07-16 at 5 20 28 PM" src="https://github.com/user-attachments/assets/0658d707-2675-4820-a57e-921ad4a886ff">

Positions are updated so they correctly highlight as you work through the function call

<img width="855" alt="Screenshot 2024-07-16 at 5 20 42 PM" src="https://github.com/user-attachments/assets/1f236056-e997-4715-be12-f82995c79959">

You'll note in the code that I have `vec_label()`, which supports both the scalar and vector case. The scalar case is pretty common, because `fn <- function(x = 1)` inlines the `1` as a double vector of length 1. The vector case is not so common. `fn <- function(x = c("a", "b"))` represents `c("a", "b")` _as a call_, so it goes through that path instead. That said, in some cases we do support inlining more complex objects, like `vctrs:::fn_inline_formals()`, so I went ahead and added support for vectors of length >1 in `vec_label()` too.

---

Related to https://github.com/posit-dev/positron/issues/1612. I think this PR is actually an easier solution, because when we look at the `formals()` of, say, `stringr::boundary()`:

```r
> formals(stringr::boundary)
$type
c("character", "line_break", "sentence", "word")

$skip_word_none
[1] NA

$...
```

we actually see `c("character", "line_break", "sentence", "word")` expressed _as a call_. So to implement the feature described there we'd have to evaluate that call, which can be tricky as many times it won't be able to be evaluated. This PR at least makes it so you can see the values without evaluating the call.

## @kevinushey at 2024-07-17T02:23:07Z

Throwing it out there: do we want to do anything to neatly truncate the signature tooltip in case of functions with a large number of arguments, or very large default arguments?

How would it feel if we only displayed the default argument values for the current / active formal argument, if any?

Some examples of functions with large formals:

- `str.default`
- `read.table`
- `install.packages`

## @DavisVaughan at 2024-07-17T11:02:18Z

> How would it feel if we only displayed the default argument values for the current / active formal argument, if any?

I originally thought that wasn't possible because I thought we only got called 1 time up front, and then the frontend managed the "active parameter" stuff based on where the user's cursor it. But it seems like we are actually getting called after each `,` and `=` keypress, so we could do your suggestion if we wanted to.

ah and of course that is because we set

```
            signature_help_provider: Some(SignatureHelpOptions {
                trigger_characters: Some(vec!["(".to_string(), ",".to_string(), "=".to_string()]),
                retrigger_characters: None,
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: None,
                },
            }),
```

## @DavisVaughan at 2024-07-17T13:33:15Z

> do we want to do anything to neatly truncate the signature tooltip in case of functions with a large number of arguments

We'd have to be very careful with this because we also add parameter offsets to each parameter (which are offsets into the `label`). And `r_signature_help()` is utilized in places besides just the `SignatureHelp` handler (like in generating custom completions for `Sys.setenv()` for example) so we'd have to be careful there too.

Like, if we add `<...>` into the `label` or something, do parameter offsets map to that position when they are too long?

---

The signatures are big but idk if they are _that_ bad since they don't stick around that long. I'm not sure it is worth the tradeoff compared to added complexity of not always having a spot in the `label` to map each parameter too

<img width="636" alt="Screenshot 2024-07-17 at 9 33 28 AM" src="https://github.com/user-attachments/assets/757a0d1d-7744-4b1e-a54f-303fe85ff04f">
<img width="582" alt="Screenshot 2024-07-17 at 9 33 39 AM" src="https://github.com/user-attachments/assets/9349f8b0-8682-40b2-9935-b8b2fc32deec">


## @DavisVaughan at 2024-08-28T19:51:00Z

Coming back to:

> do we want to do anything to neatly truncate the signature tooltip in case of functions with a large number of arguments, or very large default arguments?

I don't currently feel like truncation is necessary, even with long signatures like `install.packages()`. It just doesn't feel _that_ bad (see images above), and the alternative would mean that sometimes you'd have to fall back to the help documentation page, and that we'd have to additionally manage mapping the user's cursor position into the truncated range for highlighting the "current parameter" and that could be messy.

> How would it feel if we only displayed the default argument values for the current / active formal argument, if any?

My gut feeling is that it is more useful to see them all at once. Like, with `fill()` here even before I've put in the `data` I feel like it is useful to see the possible values for the other arguments. It's kind of hard to put into words why I feel this is more useful than only showing the values once you get to the argument itself.

<img width="583" alt="Screenshot 2024-08-28 at 3 49 31 PM" src="https://github.com/user-attachments/assets/2f34f5b9-d2b3-455a-bf5e-d20ce4f96653">


## @kevinushey at 2024-08-28T20:01:52Z

On the balance, I think you're right -- put another way, we probably shouldn't pessimize the overall design for the relatively more rare cases of functions with overly large signatures. We might still want to consider some sort of smart truncation for those cases, but even then doing nothing is probably fine too.

## @lionel- at 2024-08-29T11:42:47Z

> I don't currently feel like truncation is necessary, even with long signatures like install.packages(). It just doesn't feel that bad (see images above), and the alternative would mean that sometimes you'd have to fall back to the help documentation page

This is my feeling as well.
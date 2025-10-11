# Make optional parts of `LanguageInfo` actually optional!

> <https://github.com/posit-dev/ark/pull/553>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

I noticed in https://github.com/posit-dev/positron/issues/2098#issuecomment-2384064069 that in a jupyter console we get this pygment error:

<img width="1386" alt="Screenshot 2024-09-30 at 4 02 18 PM" src="https://github.com/user-attachments/assets/7a9c0015-5be4-4732-8cf8-73056d0ce2f6">

See that `for language ''`? That implies we are actually sending _the empty string_ over for the `pygments_lexer` part of the `language_info` part of a `kernel_info_reply`!
https://jupyter-client.readthedocs.io/en/stable/messaging.html#kernel-info

Notice that it says

```
        # Pygments lexer, for highlighting
        # Only needed if it differs from the 'name' field.
        'pygments_lexer': str,
```

I'm interpreting this as meaning that its actually an optional field. We were treating it as a required `String`, but we sent over `String::new()`.

And indeed jupyter_console treats it as an optional field!
https://github.com/jupyter/jupyter_console/blob/fddbc42d2e0be85feace1fe783a05e2b569fceae/jupyter_console/ptshell.py#L541-L542

If `pygments_lexer` is not there, it falls back to `name` of `"R"` which is exactly what we wanted all along.

---

Now for the cool part. In jupyter console, `pygments_lexer` is used to look up syntax highlighting using the pygments library. So fixing this actually enables R syntax highlighting in jupyter console for ark!

It uses this, with the key being `short name: r`
https://pygments.org/docs/lexers/#pygments.lexers.r.SLexer

<img width="538" alt="Screenshot 2024-09-30 at 4 32 06 PM" src="https://github.com/user-attachments/assets/a88b2b4a-0e91-4149-b4e2-8069273ca68c">




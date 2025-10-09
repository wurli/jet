# Support semi colons on single line in statement range detection

> <https://github.com/posit-dev/ark/pull/638>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Branched from #636.
Addresses https://github.com/posit-dev/positron/issues/4317

~Adds a dependency of statement-range on the R parser. This is consistent with our plans to make evaluation of selections depend on the R parser as well (complete expressions detection).~

~When a statement range is requested, we simply check if the whole line completely parses. If that's the case we return the line range. This tactic doesn't work if multiline expressions are separated by semi-colons. We can find a better way when we switch to a Rowan implementation.~

No longer doing this, instead using https://github.com/posit-dev/ark/pull/644, which does not use the R parser.


### Positron Release Notes

#### New Features

- N/A

#### Bug Fixes

- `Cmd/Ctrl + Enter` now works as expected with expressions separated by semi-colons on a single line.
   https://github.com/posit-dev/positron/issues/4317

### Positron QA Notes

Cmd/Ctrl + Enter on that line (or an empty preceding line) should run the entire line:

```r
1; 2; 3
```

A semi-colon following a multiline expression is also supported, so for instance this should also evaluate everything if you put your cursor on the `{` and hit Cmd + Enter

```r
{
  1
}; 2
```



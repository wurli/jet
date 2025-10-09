# Take into account user-specified log level

> <https://github.com/posit-dev/ark/pull/3>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Currently we pass `RUST_LOG` to the log macros via `set_max_level()` but not to our logger implementation. This causes macros of higher verbosity level than "info" to be excluded from the log. To fix this, we now set `LOGGER.level` to the user value during initialisation.

For simplicity I moved the initialisation block inside the `match` branch where we have access to the unwrapped level. Also we're now matching on `Level` rather than `LevelFiter` since we need both kinds of objects and the latter can be created from the former but not vice versa.


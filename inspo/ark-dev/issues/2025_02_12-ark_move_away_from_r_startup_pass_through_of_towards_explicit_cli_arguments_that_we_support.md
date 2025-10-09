# Ark: Move away from R startup pass through of `--` towards explicit CLI arguments that we support

> <https://github.com/posit-dev/ark/issues/708>
> 
> * Author: @DavisVaughan
> * State: OPEN
> * Labels: 

Right now, ark has an argument "pass through" method of `--` that passes on _any_ command line arguments to R

In Positron we set a few, like `--interactive` and `--quiet` and `--no-restore-data`, but the user has a chance to set some too:

![Image](https://github.com/user-attachments/assets/3ecd0970-40b1-43c9-9243-42f49f76b200)

This is actually absolutely not working as intended right now. On Windows, we aren't even passing these arguments through to R's cmdlinearg utility
https://github.com/posit-dev/ark/blob/d6ab389bb828db56850948f10b27e3161285be47/crates/ark/src/sys/windows/interface.rs#L55-L56

But here's the thing, I _don't think we should_. In RStudio, there is no option to just pass whatever flags you want through to R. RStudio exposes a UI to set _blessed_ options, and then it does whatever internal magic is needed to mimic things like `--no-init-file` (which would be to set `pRP->LoadInitFile = loadInitFile ? TRUE : FALSE`, not to pass `--no-init-file` through to R directly).

This is a good thing because it would put ark in control over the limited set of blessed options you can tweak while starting R. For example, we'd never want anyone to pass `-e` to run a single expression and then exit. 

Here is the set of ark options

```
--connection_file
--startup-file
--session-mode
--no-capture-streams
--version
--log
--install
--help
```

I think we should also add this explicit set of R options instead of `--`

```
--r-quiet

# These default to TRUE like R, so the flag name is `no`
--r-no-init-file
--r-no-site-file
--r-no-environ

# I vote we default these to FALSE internally, unlike R,
# which means we also remove the `"no"` from their R option name
--r-save
--r-restore-data
--r-restore-history

# Advanced user customization, sometimes for workbench / security
--r-max-connections=N
--r-max-ppsize=N
--r-min-nsize=N
--r-min-vsize=N

# Special, but common
# We would already _default_ to no-save, no-restore, so this would practically
# add no-init-file, no-site-file, no-environ
--r-vanilla
```

That is the complete list of what I think we should add as explicit options out of this set from R's help page:

```
Options:
  -h, --help            Print short help message and exit
  --version             Print version info and exit
  --encoding=ENC        Specify encoding to be used for stdin
  --encoding ENC
  RHOME			Print path to R home directory and exit
  --save                Do save workspace at the end of the session
  --no-save             Don't save it
  --no-environ          Don't read the site and user environment files
  --no-site-file        Don't read the site-wide Rprofile
  --no-init-file        Don't read the user R profile
  --restore             Do restore previously saved objects at startup
  --no-restore-data     Don't restore previously saved objects
  --no-restore-history  Don't restore the R history file
  --no-restore          Don't restore anything
  --vanilla		Combine --no-save, --no-restore, --no-site-file,
			--no-init-file and --no-environ
  --no-readline         Don't use readline for command-line editing
  --max-connections=N   Set max number of connections to N
  --max-ppsize=N        Set max size of protect stack to N
  --min-nsize=N         Set min number of fixed size obj's ("cons cells") to N
  --min-vsize=N         Set vector heap minimum to N bytes; '4M' = 4 MegaB
  -q, --quiet           Don't print startup message
  --silent              Same as --quiet
  -s, --no-echo         Make R run as quietly as possible
  --interactive         Force an interactive session
  --verbose             Print more information about progress
  -d, --debugger=NAME   Run R through debugger NAME
  --debugger-args=ARGS  Pass ARGS as arguments to the debugger
  -g TYPE, --gui=TYPE	Use TYPE as GUI; possible values are 'X11' (default)
			and 'Tk'.
  --arch=NAME		Specify a sub-architecture
  --args                Skip the rest of the command line
  -f FILE, --file=FILE  Take input from 'FILE'
  -e EXPR               Execute 'EXPR' and exit
```

## @DavisVaughan at 2024-10-11T14:30:43Z

https://github.com/clap-rs/clap/issues/3513 has an example of argument namespacing like we'd want here. something like

```
ark

USAGE:
    ark [OPTIONS] [R OPTIONS]

OPTIONS:
    -h, --help                         Print help information
    --some-string 

R OPTIONS:
        --r.quiet 
```

Do we prefer underscore or dashes? `r.no_init_file` or `r.no-init-file`? I think we are locked in to `connection_file` through jupyter, so maybe we should use underscore everywhere for consistency.

## @jennybc at 2024-10-11T14:55:45Z

The main argument I didn't see explicitly called out above (it's in the list, of course) that I think features prominently in R lore is `--vanilla`. I know it's just a combination of several specific options, but I think the `--vanilla` combo pack would be nice to expose explicitly in UI/docs as a familiar shorthand for a certain set of flags.

## @kevinushey at 2024-10-11T16:56:55Z

> Do we prefer underscore or dashes? r.no_init_file or r.no-init-file? I think we are locked in to connection_file through jupyter, so maybe we should use underscore everywhere for consistency.

I personally prefer the dashes. I can't think of any other tools which use `_` as a separator for command line arguments. I also prefer having the names match the R equivalents as closely as possible, so we shouldn't transform those separators just for internal consistency. My vote would be for `--r.no-save` and `--r.no-init-file`.

## @DavisVaughan at 2024-10-11T17:14:57Z

Oh you know what, the jupyter spec says we just need this `--connection-file {connection_file}` - i.e. the arg name can be separated by a dash as long as the templated thing has the underscore.

So I think we can fully standardize on an underscore everywhere.

Agree that `--r.no-init-file` is pretty nice for matching the R equivalent closely

## @jmcphers at 2024-10-11T20:41:44Z

> In RStudio, there is no option to just pass whatever flags you want through to R. 

There isn't, but people have repeatedly asked for it, which is why I added the passthrough to Ark originally. Some Workbench customers have asked for the ability to set e.g. `--min-vsize` and we just have to say "sorry, you can't". 

Totally fine with removing the ability to set _any_ command line arg you want, but some people do want to control allocation behavior so I'd suggest adding the max/min flags.
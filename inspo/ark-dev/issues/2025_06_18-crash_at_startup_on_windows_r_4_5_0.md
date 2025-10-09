# Crash at startup on Windows, R 4.5.0

> <https://github.com/posit-dev/ark/issues/844>
> 
> * Author: @jmcphers
> * State: OPEN
> * Labels: 

Unfortunately we don't have a repro for this, but a user has reported a consistent ark crash at startup on their Windows machine. Here's the backtrace:

```
Streaming kernel log file: C:\Users\allth\AppData\Local\Temp\kernel-49QSKD\kernel.log
[R]   2025-06-11T09:17:43.324080Z ERROR  
[R] >>> Backtrace for signal 11
[R] >>> In thread main
[R]    0: <unknown>
[R]    1: <unknown>
[R]    2: <unknown>
[R]    3: seh_filter_exe
[R]    4: tree_sitter_r
[R]    5: _C_specific_handler
[R]    6: _chkstk
[R]    7: RtlWow64GetCurrentCpuArea
[R]    8: KiUserExceptionDispatcher
[R]    9: gl_loadhistory
[R]   10: addhistory
[R]   11: rwarn_
[R]   12: R_ParseEvalString
[R]   13: R_ParseEvalString
[R]   14: R_ParseEvalString
[R]   15: Rf_eval
[R]   16: Rf_eval
[R]   17: R_ParseEvalString
[R]   18: Rf_eval
[R]   19: R_ParseEvalString
[R]   20: R_ParseEvalString
[R]   21: Rf_eval
[R]   22: R_ParseEvalString
[R]   23: Rf_eval
[R]   24: Rf_eval
[R]   25: R_ParseEvalString
[R]   26: setup_Rmainloop
[R]   27: <unknown>
[R]   28: <unknown>
[R]   29: <unknown>
[R]   30: <unknown>
[R]   31: <unknown>
[R]   32: <unknown>
[R]   33: <unknown>
[R]   34: <unknown>
[R]   35: tree_sitter_r
[R]   36: BaseThreadInitThunk
[R]   37: RtlUserThreadStart
[R] 
[R]     at crates\ark\src\traps.rs:41
```

Platform info:

```
Positron Version: 2025.07.0 (system setup) build 80
Code - OSS Version: 1.100.0
Commit: c1b9c44024d43c164af548c2d0f916f18e552bdc
Date: 2025-06-10T03:51:36.729Z
Electron: 34.5.1
Chromium: 132.0.6834.210
Node.js: 20.19.0
V8: 13.2.152.41-electron.0
OS: Windows_NT x64 10.0.26100
```

Original thread: https://github.com/posit-dev/positron/discussions/8008#discussioncomment-13430987




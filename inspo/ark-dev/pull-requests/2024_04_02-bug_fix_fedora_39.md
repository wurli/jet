# Fix compilation on fedora 39 by updating zeromq-src

> <https://github.com/posit-dev/ark/pull/292>
> 
> * Author: @wesm
> * State: MERGED
> * Labels: 

This was failing for me with zeromq-src 0.2.5 and fixed by the point version update:

```
  cargo:warning=In file included from /home/wesm/.cargo/registry/src/index.crates.io-6f17d22bba15001f/zeromq-src-0.2.5+4.3.4/vendor/src/ws_engine.cpp:57:
  cargo:warning=/home/wesm/.cargo/registry/src/index.crates.io-6f17d22bba15001f/zeromq-src-0.2.5+4.3.4/vendor/src/compat.hpp:45:1: error: ‘size_t strlcpy(char*, const char*, size_t)’ was declared ‘extern’ and later ‘static’ [-fpermissive]
  cargo:warning=   45 | strlcpy (char *dest_, const char *src_, const size_t dest_size_)
  cargo:warning=      | ^~~~~~~
  cargo:warning=In file included from /usr/include/c++/13/cstring:42,
  cargo:warning=                 from /home/wesm/.cargo/registry/src/index.crates.io-6f17d22bba15001f/zeromq-src-0.2.5+4.3.4/vendor/src/ws_engine.cpp:55:
  cargo:warning=/usr/include/string.h:506:15: note: previous declaration of ‘size_t strlcpy(char*, const char*, size_t)’
  cargo:warning=  506 | extern size_t strlcpy (char *__restrict __dest,
```


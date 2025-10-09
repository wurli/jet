# Compile portably for x64 and arm64 Linux

> <https://github.com/posit-dev/ark/pull/486>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/4506
Addresses the ark side of https://github.com/posit-dev/positron/issues/3854

Edit: Addresses https://github.com/posit-dev/positron/issues/4507

Solves linking issues for:

- Open SSL by statically linking to libssl. See https://github.com/posit-dev/positron/issues/3854#issuecomment-2226860016 for an example of linking issue. Static linking is supported by the `openssl-sys` crate, though it's slightly tricky as it expects us to provide the location of the static lib and headers in a single folder. The system installation of libssl on Ubuntu puts headers in two folders, so we merge them as a workaround.

  Another approach supported by `openssl-sys` is to compile Open SSL as part of the build ("vendored" crate feature), but I had trouble getting it to work. Might be worth looking into further for arm64 support though.

- GNU libc by using the zig linker with https://github.com/rust-cross/cargo-zigbuild. This linker supports linking to specific versions of glibc: https://github.com/ziglang/glibc-abi-tool. We use this to target glibc 2.26, which is sufficiently old to run on RHEL8 or OpenSUSE 15.2.

Using these approaches we now get a pretty portable build of ark.

I thought that we'd also have to solve linking issues with the C++ runtime but that doesn't seem to be the case. In fact I no longer see libstdc++ in the output of `ldd`, which I find very surprising:

```
ldd ark
	linux-vdso.so.1 (0x00007fffd3d71000)
	libm.so.6 => /lib/x86_64-linux-gnu/libm.so.6 (0x00007f6f95b19000)
	libpthread.so.0 => /lib/x86_64-linux-gnu/libpthread.so.0 (0x00007f6f973a1000)
	libc.so.6 => /lib/x86_64-linux-gnu/libc.so.6 (0x00007f6f95800000)
	libdl.so.2 => /lib/x86_64-linux-gnu/libdl.so.2 (0x00007f6f9739c000)
	/lib64/ld-linux-x86-64.so.2 (0x00007f6f973b3000)
```

I verified that the ark binary produced by https://github.com/posit-dev/ark/actions/runs/10591447650 works on the following fuzzbucket platforms:

```sh
fuzzbucket-client create ubuntu20-ide-prereqs-ci
fuzzbucket-client create ubuntu22-ide-prereqs
fuzzbucket-client create ubuntu24-ide-prereqs-ci
fuzzbucket-client create rhel8-ide-prereqs
fuzzbucket-client create rhel9-ide-prereqs
fuzzbucket-client create opensuse15.2
fuzzbucket-client create opensuse15.5-ide-prereqs-ci
```

Testing method: Connecting with jupyter-console and running `rnorm(1)`.

I removed the debug build from our matrix because statically linking to openssl with debug symbols causes the size of the build to explode to over 230mb. If debug symbols are needed, getting a local build of ark going is a simple matter of `git clone` + `rustup` + `cargo build`, which seems reasonably easy?

## @kevinushey at 2024-08-28T16:53:48Z

IMHO static linking to OpenSSL and libc is a bad idea, unless we're certain it's trivial to update ark (e.g. if a user of Positron / ark needs to update in response to security vulnerabilities discovered in OpenSSL).

## @jmcphers at 2024-08-28T17:18:14Z

This is super cool, but unfortunately I agree with @kevinushey. A portable build would be really convenient, but becoming an OpenSSL distributor (by statically linking it) would be a nightmare. https://openssl-library.org/news/vulnerabilities/index.html

Here are four other routes we could take:

- Look up OpenSSL symbols dynamically like we do for R. May or may not be possible; I'm aware that most OpenSSL usage may be in crates we do not own. We've discussed doing this for RStudio for years.
- Attempt to remove OpenSSL as a dependency entirely. Ark doesn't have a lot of crypto or use any TLS/SSH/etc. features. Maybe we could use alternatives for the few crypto-related features we have that don't require OpenSSL.
- Produce separate builds of Ark that link against the appropriate OpenSSL for each Linux distribution (minimally by ABI compatibility)
- [Run away from home and live in the woods](https://www.wikihow.com/Run-Away-from-Home-and-Live-in-the-Woods)

Producing separate builds is probably the easiest way forward and it's what we have always done to fix this problem. But I think it would also be worth investigating where the dependency is coming from and seeing if there are alternatives. 

## @DavisVaughan at 2024-08-28T17:39:47Z

@jmcphers re bullet 2, like this? https://github.com/posit-dev/ark/pull/487

## @lionel- at 2024-08-29T07:33:57Z

oh I had assumed that ssl was used somewhere in our transport stack...

I just found out it's very easy to analyse dependency chains:

```
% cargo tree --target all --no-dedupe -i openssl-sys
openssl-sys v0.9.93
├── native-tls v0.2.11
│   ├── hyper-tls v0.5.0
│   │   └── reqwest v0.11.22
│   │       └── ark v0.1.126 (/Users/lionel/Sync/Projects/Positron/ark/crates/ark)
│   ├── reqwest v0.11.22
│   │   └── ark v0.1.126 (/Users/lionel/Sync/Projects/Positron/ark/crates/ark)
│   └── tokio-native-tls v0.3.1
│       ├── hyper-tls v0.5.0
│       │   └── reqwest v0.11.22
│       │       └── ark v0.1.126 (/Users/lionel/Sync/Projects/Positron/ark/crates/ark)
│       └── reqwest v0.11.22
│           └── ark v0.1.126 (/Users/lionel/Sync/Projects/Positron/ark/crates/ark)
└── openssl v0.10.57
    └── native-tls v0.2.11
        ├── hyper-tls v0.5.0
        │   └── reqwest v0.11.22
        │       └── ark v0.1.126 (/Users/lionel/Sync/Projects/Positron/ark/crates/ark)
        ├── reqwest v0.11.22
        │   └── ark v0.1.126 (/Users/lionel/Sync/Projects/Positron/ark/crates/ark)
        └── tokio-native-tls v0.3.1
            ├── hyper-tls v0.5.0
            │   └── reqwest v0.11.22
            │       └── ark v0.1.126 (/Users/lionel/Sync/Projects/Positron/ark/crates/ark)
            └── reqwest v0.11.22
                └── ark v0.1.126 (/Users/lionel/Sync/Projects/Positron/ark/crates/ark)
```

## @lionel- at 2024-08-29T07:37:39Z

I had just assumed that ark, by way of running R code, is a security gap in itself.

@jmcphers @kevinushey Do the security concerns apply equally to libc? What about the Rust runtime and all the Rust dependencies statically linked to ark?

Edit: I guess Rust is much safer than C in general which could make these less of a concern.

Edit2: oh but the static linking concerns do not apply here. We are _dynamically_ linking to libc, against a specific version of the ABI. This is different from approaches that use e.g. musl to statically compile against a C runtime. Since we link dynamically, security updates installed on the host will apply as normal. So we should be good?

My main question is whether we can use the zigbuild trick to link to old GNU libc versions?

## @lionel- at 2024-08-29T09:23:04Z

- Now branched from @DavisVaughan's PR #487. We no longer depend on libssl at all.

- With that out of the way it was easy to cross-compile for arm, adresses posit-dev/positron#4507.

## @lionel- at 2024-08-29T19:02:27Z

@jmcphers It's dynamically linked but against our target version of libc. So it does pretty much what we need it to AFAICT.

## @jmcphers at 2024-08-29T19:09:03Z

great!

## @lionel- at 2024-08-29T22:30:29Z

If you're curious how it works, see the glibc section in https://andrewkelley.me/post/zig-cc-powerful-drop-in-replacement-gcc-clang.html (the whole post is worth a read).

---

Now I was wondering why libstdc++ disappeared from the `ldd` output. It turns out that zig is not only used for linking together compiled object files, it's also used to compile C and C++ files. This means zeromq is now compiled by zig via llvm.

While glibc is linked dynamically for C code, by default C++ code gets statically linked with llvm-libcxx to avoid ABI issues. There seems to be a way to dynamically link to libstdc++ instead, but:

- This might be tricky to set up via `cargo zigbuild`.
- We might get into ABI compat issues.
- We already statically link to the Rust runtime and all our Rust dependencies, so surely we're way more exposed via the Rust stack. But there might be process/political reasons why llvm-libcxx is considered more critical?

@jmcphers What do you think? Do we need to find another way?

## @lionel- at 2024-09-10T06:14:22Z

A search for libcxx vulnerabilities shows very few hits compared to glibc, so it seems much safer to distribute as part of ark (not that we could statically link to glibc if we wanted to, just using it as a point of comparison).

Regarding the licence, it's apache 2.0 with restrictions, but it seems fine:

> https://llvm.org/docs/DeveloperPolicy.html#new-llvm-project-license-framework
In particular:
Binaries that include LLVM must reproduce the copyright notice (e.g. in an included README file or in an “About” box), unless the LLVM code was added as a by-product of compilation. For example, if an LLVM runtime library like compiler_rt or libc++ was automatically included into your application by the compiler, you do not need to attribute it.

See also MS comments in https://devblogs.microsoft.com/cppblog/open-sourcing-msvcs-stl:

> As a customer of MSVC’s STL, you might be wondering whether this new license creates new obligations for you. Microsoft’s position is that the text of the Apache License v2.0 with LLVM Exceptions (specifically, the wording of the LLVM Exceptions) clearly states that when you compile your own source code using MSVC’s STL to produce object code or similar output (e.g. static libraries, dynamic libraries, and executables), you aren’t required to provide attribution when shipping your compiled products to your end users. This is another reason we’ve chosen this license: to avoid disrupting our customers’ businesses.

In light of this, we've decided to go with the zig approach as that allows us to compile portably for all supported distributions on both x86_64 and arm64.
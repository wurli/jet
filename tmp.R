# Requires cli >=3.6.1.9000 (https://github.com/r-lib/cli/pull/625)
options(cli.default_num_colors = 256L)
options(cli.dynamic = TRUE)
options(cli.ansi = TRUE)
options(cli.hyperlink = TRUE)
options(cli.hyperlink_run = TRUE)
options(cli.hyperlink_help = TRUE)
options(cli.hyperlink_vignette = TRUE)

# 3. File a polite PR upstream against kallichore.json to fix the OpenAPI 3.0 violations (1) and (2) —
# standardising the spec helps everyone, and our build.rs fixups become inert (still safe to keep as
# defence-in-depth).
# 4. Pin kallichore to a tagged release rather than main once posit-dev cuts one — 9ca5338 is just whatever
# main happened to be today.
# 5. Add a CI step cargo build && git diff --exit-code src/kallichore/generated.rs to catch
# hand-edits or undeclared spec changes.

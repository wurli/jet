
# Requires cli >=3.6.1.9000 (https://github.com/r-lib/cli/pull/625)
options(cli.default_num_colors = 256L)
# Tell cli that Positron's console supports dynamic updates
# TODO: This would be better as `cli.default_dynamic`, but that doesn't exist
# yet.
options(cli.dynamic = TRUE)
# Tell cli what kind of hyperlinks are supported in the Positron console.
# TODO: These would be better as `cli.default_*`, but those don't exist yet.
options(cli.hyperlink = TRUE)
options(cli.hyperlink_run = TRUE)
options(cli.hyperlink_help = TRUE)
options(cli.hyperlink_vignette = TRUE)

print("bla")

library(tidyverse)


tibble(x = 1:10, y = rnorm(10))

economics

stop("oh no")

for (i in 1:10) {
  Sys.sleep(1)
  print(sample(stringr::words, 1))
}

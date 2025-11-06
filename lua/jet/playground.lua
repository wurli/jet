Jet = require("jet.rust")
Kernel = require("jet.kernel")
Ark = Kernel.new("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/ark/kernel.json")

Ark:execute("options(cli.num_colors = 256)")
Ark:execute("dplyr::tibble(x = 1:5, y = rnorm(5))")
Ark:execute("for (i in 1:3) {Sys.sleep(0.5); print(i)}")


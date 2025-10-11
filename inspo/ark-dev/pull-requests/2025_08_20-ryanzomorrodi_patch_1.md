# Exit after --version or --help

> <https://github.com/posit-dev/ark/pull/902>
>
> * Author: @ryanzomorrodi
> * State: MERGED
> * Labels:

Closes #863. I think it makes sense to exit after `--version` or `--help` is used.

It's pretty typical behavior for cli tools like:
```bash
quarto --version --help
#> 1.7.32
```

```bash
quarto --help --version
#>
#> Usage:   quarto
#> Version: 1.7.32
#>
#> Description:
#>
#>   Quarto CLI
#>
#> Options:
#>
#>   -h, --help     - Show this help.
#>   -V, --version  - Show the version number for this program.
#>
#> Commands:
#>
#>   render     [input] [args...]     - Render files or projects to various document types.
#>   preview    [file] [args...]      - Render and preview a document or website project.
#>   serve      [input]               - Serve a Shiny interactive document.
#>   create     [type] [commands...]  - Create a Quarto project or extension
#>   use        <type> [target]       - Automate document or project setup tasks.
#>   add        <extension>           - Add an extension to this folder or project
#>   update     [target...]           - Updates an extension or global dependency.
#>   remove     [target...]           - Removes an extension.
#>   convert    <input>               - Convert documents to alternate representations.
#>   pandoc     [args...]             - Run the version of Pandoc embedded within Quarto.
#>   typst      [args...]             - Run the version of Typst embedded within Quarto.
#>   run        [script] [args...]    - Run a TypeScript, R, Python, or Lua script.
#>   list       <type>                - Lists an extension or global dependency.
#>   install    [target...]           - Installs a global dependency (TinyTex or Chromium).
#>   uninstall  [tool]                - Removes an extension.
#>   tools                            - Display the status of Quarto installed dependencies
#>   publish    [provider] [path]     - Publish a document or project to a provider.
#>   check      [target]              - Verify correct functioning of Quarto installation.
#>   call                             - Access functions of Quarto subsystems such as its rendering engines.
#>   help       [command]             - Show this help or the help of a sub-command.
```

## @github-actions at 2025-08-20T15:26:41Z

All contributors have signed the CLA  ✍️ ✅<br/><sub>Posted by the ****CLA Assistant Lite bot****.</sub>

## @ryanzomorrodi at 2025-08-20T15:29:28Z

I have read the CLA Document and I hereby sign the CLA

## @lionel- at 2025-08-21T11:49:41Z

Thanks!

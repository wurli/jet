# Hard-to-read tables in markdown help pages

> <https://github.com/posit-dev/ark/issues/744>
> 
> * Author: @wurli
> * State: OPEN
> * Labels: 

Hi :)

I've been toying with integrating Ark with Neovim via [ark.nvim](https://github.com/wurli/ark.nvim). Happily, my experience so far has been really good. However, one minor annoyance is that LSP formatting of 'Arguments' sections is a bit hard to read:

<img width="1268" alt="Image" src="https://github.com/user-attachments/assets/1dbae10c-30f1-49df-af1e-192a6f8f0724" />

I *think* Positron requests HTML from Ark, rather than requesting markdown and converting to HTML downstream. If this is the case, would you be open to a pull request to change the formatting here, just for the markdown option, e.g. to use a bullet list or similar?

I'm very aware that ark.nvim is (currently) something of an off-label use for Ark, as the README clearly states that the LSP/DAP servers aren't yet available for other editors. With this in mind, I also wanted to ask if Ark is currently open to issues/contributions like this one, which are aimed at making Ark more ergonomic for other editors? If not, please do let me know and I'll be happy to hold off with any similar questions until that changes. 

Thank you!




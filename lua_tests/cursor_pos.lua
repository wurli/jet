

-- Nice; vim.fn.strchars() gives the correct cursor position.
vim.print(vim.fn.strchars("𨭎𨭎𨭎𨭎𨭎"))
vim.print(vim.fn.strlen("𨭎𨭎𨭎𨭎𨭎"))
vim.print(("𨭎𨭎𨭎𨭎𨭎"):len())


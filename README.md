# Dwarf2Ctags
Extract the function locations from an elf file to ctags, 

this is useful when you have an existing project that you want to quickly navigate but setting up better code navigation will take to long.

Tested using an elf file generated by a Zephyr RTOS C project, on Windows with neovim.


# Instructions
checkout the repo
> cargo build

> cargo run <path_to_your_elf_file>

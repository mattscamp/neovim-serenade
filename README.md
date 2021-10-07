# neovim-serenade
> Program by voice in Neovim with Serenade

# Installation
Use your preferred plugin manager. Run `install.sh` as a post-installation step, which will download and install the pre-built binary.

For example, for `vim-plug`, you can put in the following line into your `.vimrc`:
```vim
Plug 'mattscamp/neovim-serenade', { 'do': 'bash install.sh' }
```

# Usage
Refer to the following table to find supported commands.

| Command                    | Description                                               |
|----------------------------|-----------------------------------------------------------|
| `:SerenadeStop`            | Stop listening for Serenade commands                      |
| `:SerenadeStart`           | Start listening for Serenade commands (listens by defualt)|

# License
MIT

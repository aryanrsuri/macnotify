# SSH, tmux, and Neovim theme integration

This optional example follows macOS appearance on a Linux SSH server. It is separate from Pubrat's native notification and command-running code.

```text
Pubrat (Mac events)
  → sync-theme.sh (local update + SSH)
    → remote term-theme (saved mode + tmux)
      → ~/.config/nvim/refresh-theme.sh (RPC to running Neovim)
        → ~/.config/nvim/lua/terminal_theme.lua (Neovim colors)
```

## Files and ownership

| File | Install on | Purpose |
| --- | --- | --- |
| `sync-theme.sh` | Mac: `~/.local/bin/sync-theme` | Run the local theme command, read macOS appearance, and call the server. |
| `remote-term-theme.sh` | Server: `~/.local/bin/term-theme` | Select `light` or `dark`, apply tmux colors, and delegate editor refresh. |
| `nvim/lua/terminal_theme.lua` | Server: `~/.config/nvim/lua/terminal_theme.lua` | Read the selected mode and apply the `reference` colorscheme. |
| `nvim/refresh-theme.sh` | Server: `~/.config/nvim/refresh-theme.sh` | Notify running Neovim instances through RPC. |

The local Mac `~/.local/bin/term-theme` is supplied by you. It should apply the current macOS appearance locally and return nonzero on failure. The adapter attempts the remote update even if that local command fails.

## Server prerequisites

Supply your own matching light/dark palettes:

```text
~/.config/terminal-style/
  light/theme.tmux
  dark/theme.tmux
  current -> light or dark
~/.config/nvim/colors/reference.lua
```

The Neovim module uses `reference`; change that name to your own colorscheme when adapting the example. Its colorscheme must support `vim.o.background = "light"` and `"dark"`.

The server needs `tmux`, `nvim` with RPC support, and GNU `timeout` from coreutils. The refresh script discovers default Neovim sockets under `$XDG_RUNTIME_DIR`, falling back to Linux `/run/user/$(id -u)`. Custom `--listen` socket locations require adjusting the script.

## Install the server integration

Run from a checkout of this repository on the Linux server:

```sh
mkdir -p "$HOME/.local/bin" "$HOME/.config/nvim/lua"
install -m 755 examples/ssh-theme/remote-term-theme.sh "$HOME/.local/bin/term-theme"
install -m 644 examples/ssh-theme/nvim/lua/terminal_theme.lua "$HOME/.config/nvim/lua/terminal_theme.lua"
install -m 755 examples/ssh-theme/nvim/refresh-theme.sh "$HOME/.config/nvim/refresh-theme.sh"
```

Add this once to `~/.config/nvim/init.lua`, replacing any older duplicate theme-refresh autocmd:

```lua
require("terminal_theme").setup()
```

For existing editor instances, load the new file once directly (plugin loaders may have cached the previously absent module):

```vim
:lua package.loaded.terminal_theme = dofile(vim.fn.stdpath("config") .. "/lua/terminal_theme.lua"); require("terminal_theme").setup()
```

Future instances load it from `init.lua`. Test the server command:

```sh
~/.local/bin/term-theme light
~/.local/bin/term-theme dark
```

The remote command invokes Neovim's refresh script immediately. The module also refreshes on focus and buffer changes. No editor keystrokes are injected, and editors are not restarted.

## Install the Mac adapter

From the Mac checkout:

```sh
mkdir -p "$HOME/.local/bin"
install -m 755 examples/ssh-theme/sync-theme.sh "$HOME/.local/bin/sync-theme"
THEME_SSH_HOST=your-server "$HOME/.local/bin/sync-theme"
```

`your-server` is an SSH host alias from your own configuration. Noninteractive authentication and host-key trust must already work. The adapter uses SSH batch mode plus connection and liveness timeouts; these do not impose a deadline on a remote command that stays connected but hangs.

Use the generic [`../LaunchAgent.plist`](../LaunchAgent.plist), with these two `ProgramArguments` values after replacing the username:

```xml
<string>/Users/YOUR_USER/.local/bin/theme-listener</string>
<string>/Users/YOUR_USER/.local/bin/sync-theme</string>
```

Add this inside its existing `EnvironmentVariables` dictionary:

```xml
<key>THEME_SSH_HOST</key>
<string>your-server</string>
```

Follow the [root README's startup instructions](../../README.md#start-at-login) to bootstrap the agent. Stop and bootstrap it again after editing its environment or arguments.

## Verify the complete path

Switch actual macOS appearance to dark, then light. For each transition, verify the server's `~/.config/terminal-style/current`, tmux colors, and `:set background?` inside each running Neovim instance. Do not rely only on a synthetic notification test.

A failed command causes Pubrat to retry with backoff; successful commands leave it idle. Codex can retain cached terminal colors in an existing session. This integration never restarts Codex; refresh or restart that application manually when needed.

#!/bin/sh
# Linux server adapter; terminal palettes are controlled by the Mac.
set -eu
styles="$HOME/.config/terminal-style"
case "${1:-}" in
    light|dark) ln -sfn "$1" "$styles/current" ;;
    '') readlink "$styles/current"; exit 0 ;;
    *) printf 'Usage: term-theme [light|dark]\n' >&2; exit 2 ;;
esac
if tmux list-sessions >/dev/null 2>&1; then
    tmux source-file "$styles/current/theme.tmux"
fi

# Neovim owns its application-specific refresh implementation.
exec "$HOME/.config/nvim/refresh-theme.sh"

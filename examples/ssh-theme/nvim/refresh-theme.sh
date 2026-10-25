#!/bin/sh
# Linux: refresh each running Neovim through its default RPC socket.
set -u
result=0
for socket in "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"/nvim.*; do
    [ -S "$socket" ] || continue
    timeout 2 nvim --server "$socket" --remote-expr \
        'execute("lua require(\"terminal_theme\").apply()")' >/dev/null || result=1
done
exit "$result"

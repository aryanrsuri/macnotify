#!/bin/sh
# Example adapter: supply your own local and remote term-theme commands.
set -u
: "${THEME_SSH_HOST:?Set THEME_SSH_HOST to an SSH host alias}"

result=0
"$HOME/.local/bin/term-theme" || result=1

appearance=$(/usr/bin/defaults read -g AppleInterfaceStyle 2>/dev/null || true)
case "$appearance" in
    Dark) mode=dark ;;
    *) mode=light ;;
esac

/usr/bin/ssh -n -T \
    -o BatchMode=yes \
    -o ConnectTimeout=5 \
    -o ConnectionAttempts=1 \
    -o ServerAliveInterval=5 \
    -o ServerAliveCountMax=1 \
    "$THEME_SSH_HOST" "\"\$HOME/.local/bin/term-theme\" '$mode'" || result=1

exit "$result"

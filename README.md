# macos-theme-sync

Run a command when macOS appearance changes, at startup, and after wake. Written in Rust with no third-party crates; it links directly to macOS frameworks.

The listener sleeps between events. It does not periodically read preferences or open SSH connections. Failed commands retry after 5, 10, 20, 40, 80, 160, then 300 seconds, with subsequent retries capped at 300 seconds. A new event interrupts a pending retry delay. Commands run serially, and queued events are coalesced.

## Build and run

Requires macOS, Rust with edition 2024 support, and Xcode Command Line Tools.

```sh
cargo test
cargo build --release
./target/release/theme-listener /absolute/path/to/your-theme-command
```

Arguments after the command are passed directly, without shell expansion:

```sh
./target/release/theme-listener /absolute/path/to/term-theme --sync
```

The command receives no appearance argument. Read the current appearance inside the command, so retries and coalesced events always apply the latest state. Return zero on success and nonzero to request a retry. Commands should be idempotent and terminate; the listener does not impose a command deadline.

## SSH example

[`examples/sync-theme.sh`](examples/sync-theme.sh) calls a local `~/.local/bin/term-theme`, reads the Mac's appearance, and calls the remote `~/.local/bin/term-theme light|dark` over SSH. These theme commands are supplied by you; this project does not configure terminal palettes.

The example attempts both local and remote updates even when the local update fails. SSH uses batch mode and connection/liveness timeouts. Confirm noninteractive authentication and the server's host key before running it unattended.

```sh
THEME_SSH_HOST=your-server ./target/release/theme-listener /bin/sh "$PWD/examples/sync-theme.sh"
```

## Start at login

1. Install the binary and example hook:

   ```sh
   mkdir -p "$HOME/.local/bin"
   install -m 755 target/release/theme-listener "$HOME/.local/bin/theme-listener"
   install -m 755 examples/sync-theme.sh "$HOME/.local/bin/sync-theme"
   ```

2. Copy [`examples/LaunchAgent.plist`](examples/LaunchAgent.plist) to `~/Library/LaunchAgents/local.macos-theme-sync.plist`. Replace every `/Users/YOUR_USER` placeholder and set `THEME_SSH_HOST` to your SSH host alias. LaunchAgent arguments require absolute paths; `~` is not expanded.

3. Load the agent:

   ```sh
   plutil -lint "$HOME/Library/LaunchAgents/local.macos-theme-sync.plist"
   launchctl bootstrap "gui/$(id -u)" "$HOME/Library/LaunchAgents/local.macos-theme-sync.plist"
   launchctl print "gui/$(id -u)/local.macos-theme-sync"
   ```

To stop it:

```sh
launchctl bootout "gui/$(id -u)/local.macos-theme-sync"
```

Move the plist out of `~/Library/LaunchAgents` to prevent it loading at the next login. Disable any older polling job that performs the same updates.

## Behavior and limits

- Appearance changes use `AppleInterfaceThemeChangedNotification`, an undocumented macOS notification. Wake uses `NSWorkspaceDidWakeNotification`. Verify behavior after major macOS upgrades.
- There is no network-reconnect listener. A pending failed SSH update retries automatically; a previously successful update waits for the next appearance, wake, or startup event.
- Events arriving while a command runs queue one follow-up execution. A hung command blocks later commands, but not notification delivery.
- This does not restart editors or other applications. Applications that cache colors at startup still need their normal manual refresh or restart.
- `cargo test` runs the scheduling tests on macOS or Linux. The native listener only runs on macOS; Linux tests cannot validate macOS notifications.

The repository does not yet specify a license. Choose one before distributing it for reuse.

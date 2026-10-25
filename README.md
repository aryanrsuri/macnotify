# pubrat

Run any command when macOS appearance changes, at startup, and after wake. Pubrat is written in Rust with no third-party crates; it links directly to macOS frameworks.

Pubrat knows when to run a command. Your command decides what to do. SSH, terminal palettes, tmux, and Neovim integration live in the optional [`examples/ssh-theme/`](examples/ssh-theme/) directory, outside the notifying code.

## Build and run

Requires macOS, Rust with edition 2024 support, and Xcode Command Line Tools.

```sh
cargo test
cargo build --release
./target/release/theme-listener /absolute/path/to/your-command arg1 arg2
```

The binary is named `theme-listener`. To run a shell script:

```sh
./target/release/theme-listener /bin/bash /absolute/path/to/your-script.sh
```

Arguments are passed directly, without shell expansion. The command receives no event or appearance argument. Read the current state inside the command so retries and coalesced events apply the latest state. Return zero on success and nonzero to request a retry.

## Core and integrations

| Location | Responsibility |
| --- | --- |
| `src/native.rs` | Detect startup, application appearance changes, and wake through macOS APIs. |
| `src/worker.rs` | Run queued work serially, coalesce pending events, and schedule failure-only retries. |
| `src/main.rs` | Accept a command and arguments, connect notifications to the worker, and launch the command. |
| `examples/LaunchAgent.plist` | Run any supplied command through Pubrat at login. |
| `examples/ssh-theme/` | Optional Mac → SSH server → tmux/Neovim integration, with its own setup instructions. |

There is no rules-file parser, event-selection option, or built-in remote host. Each process runs one supplied command for all three triggers. Commands can coordinate multiple actions themselves.

## Start at login

1. Install the binary:

   ```sh
   mkdir -p "$HOME/.local/bin"
   install -m 755 target/release/theme-listener "$HOME/.local/bin/theme-listener"
   ```

2. Copy [`examples/LaunchAgent.plist`](examples/LaunchAgent.plist) to `~/Library/LaunchAgents/local.pubrat.plist`. Replace `/Users/YOUR_USER` and `/absolute/path/to/your-command`. Add one `<string>` per command argument under `ProgramArguments`. Paths must be absolute; `~` is not expanded. Configure any command-specific environment variables under `EnvironmentVariables`.

3. Load and inspect the agent:

   ```sh
   plutil -lint "$HOME/Library/LaunchAgents/local.pubrat.plist"
   launchctl bootstrap "gui/$(id -u)" "$HOME/Library/LaunchAgents/local.pubrat.plist"
   launchctl print "gui/$(id -u)/local.pubrat"
   ```

To stop it:

```sh
launchctl bootout "gui/$(id -u)/local.pubrat"
```

Move the plist out of `~/Library/LaunchAgents` to prevent it loading at the next login. After changing its command or environment, stop and bootstrap it again. Disable older jobs that perform the same actions.

## Behavior and limits

- Appearance changes observe `NSApplication.effectiveAppearance` through AppKit's event loop. Wake uses `NSWorkspaceDidWakeNotification`. Startup, appearance, and wake events are logged to stderr.
- The listener sleeps between events. It does not poll preferences, open network connections, or create a retry timer after success.
- Failed commands retry after 5, 10, 20, 40, 80, 160, then 300 seconds, with subsequent retries capped at 300 seconds. A new event interrupts a pending retry delay. There is no network-reconnect listener.
- Commands run serially; events arriving during a command queue one follow-up execution. Commands should be idempotent and terminate. A hung command blocks later executions; Pubrat does not impose a command deadline.
- Pubrat does not restart applications. Its only action is invoking the supplied command.

## Validation

`cargo test` exercises retry and idle scheduling on macOS or Linux. The native listener only runs on macOS; Linux tests cannot validate notification delivery.

After installing the agent, switch the actual macOS appearance from light to dark and back. Check the event log and the command's result after both transitions. Posting a synthetic notification does not prove that macOS delivers real appearance changes. Also verify startup and wake behavior for your integration.

The repository does not yet specify a license. Choose one before distributing it for reuse.

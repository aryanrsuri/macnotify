use std::{ffi::OsString, time::Duration};

pub const USAGE: &str = "Usage: macnotify [--retry SECONDS,...] [--] COMMAND [ARG ...]\nRun on startup, appearance changes, and wake. No retries by default.\n--retry 5,15 retries twice: 5 seconds after failure, then 15 seconds after the next failure.";

pub struct Options {
    pub command: Vec<OsString>,
    pub retries: Vec<Duration>,
}

pub fn parse(mut command: Vec<OsString>) -> Result<Options, String> {
    let mut retries = Vec::new();
    if command.first().is_some_and(|arg| arg == "--retry") {
        let value = command
            .get(1)
            .and_then(|arg| arg.to_str())
            .ok_or("--retry needs comma-separated positive seconds")?;
        for part in value.split(',') {
            let seconds = part
                .parse::<u64>()
                .ok()
                .filter(|seconds| *seconds > 0)
                .ok_or("--retry needs comma-separated positive seconds")?;
            retries.push(Duration::from_secs(seconds));
        }
        command.drain(..2);
    }
    if command.first().is_some_and(|arg| arg == "--") {
        command.remove(0);
    } else if command
        .first()
        .is_some_and(|arg| arg.as_encoded_bytes().starts_with(b"-"))
    {
        return Err("unknown option; use -- before a command beginning with '-'".into());
    }
    if command.is_empty() {
        return Err("a command is required".into());
    }
    Ok(Options { command, retries })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn default_does_not_retry_or_parse_command_arguments() {
        let options = parse(args(&["script", "--retry", "invalid"])).unwrap();
        assert!(options.retries.is_empty());
        assert_eq!(options.command, args(&["script", "--retry", "invalid"]));
    }

    #[test]
    fn parses_retry_delays_and_preserves_arguments() {
        let options = parse(args(&["--retry", "5,15", "--", "script", "two words"])).unwrap();
        assert_eq!(
            options.retries,
            [Duration::from_secs(5), Duration::from_secs(15)]
        );
        assert_eq!(options.command, args(&["script", "two words"]));
    }

    #[test]
    fn rejects_invalid_options_before_starting() {
        for input in [
            vec![],
            vec!["--retry"],
            vec!["--retry", "5,15"],
            vec!["--retry", "0", "script"],
            vec!["--retry", "5,", "script"],
            vec!["--retry", "bad", "script"],
            vec!["--unknown", "script"],
        ] {
            assert!(parse(args(&input)).is_err(), "{input:?}");
        }
    }
}

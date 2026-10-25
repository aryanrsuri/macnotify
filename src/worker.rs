use std::{
    sync::mpsc::{Receiver, RecvTimeoutError},
    time::Duration,
};

// Block indefinitely while healthy. Only failed commands create a retry timer.
pub fn run(
    receiver: Receiver<()>,
    mut action: impl FnMut() -> bool,
    initial: Duration,
    maximum: Duration,
) {
    let mut retry = None;
    let mut delay = initial;
    loop {
        match retry {
            None => {
                if receiver.recv().is_err() {
                    return;
                }
            }
            Some(timeout) => match receiver.recv_timeout(timeout) {
                Ok(()) => delay = initial,
                Err(RecvTimeoutError::Timeout) => (),
                Err(RecvTimeoutError::Disconnected) => return,
            },
        }
        // Coalesce events already queued; an event during the command stays pending.
        while receiver.try_recv().is_ok() {}
        if action() {
            retry = None;
            delay = initial;
        } else {
            retry = Some(delay);
            delay = delay.saturating_mul(2).min(maximum);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::mpsc, thread};

    #[test]
    fn retries_failure_then_stays_idle_until_event() {
        let (events, receiver) = mpsc::sync_channel(1);
        let (calls, observed) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut count = 0;
            run(
                receiver,
                || {
                    count += 1;
                    calls.send(count).unwrap();
                    count != 1
                },
                Duration::from_millis(10),
                Duration::from_millis(20),
            );
        });
        events.send(()).unwrap();
        assert_eq!(observed.recv_timeout(Duration::from_secs(1)).unwrap(), 1);
        assert_eq!(observed.recv_timeout(Duration::from_secs(1)).unwrap(), 2);
        assert!(observed.recv_timeout(Duration::from_millis(60)).is_err());
        events.send(()).unwrap();
        assert_eq!(observed.recv_timeout(Duration::from_secs(1)).unwrap(), 3);
        drop(events);
        worker.join().unwrap();
    }

    #[test]
    fn new_event_interrupts_retry_delay() {
        let (events, receiver) = mpsc::sync_channel(1);
        let (calls, observed) = mpsc::channel();
        let worker = thread::spawn(move || {
            run(
                receiver,
                || {
                    calls.send(()).unwrap();
                    false
                },
                Duration::from_secs(60),
                Duration::from_secs(300),
            );
        });
        events.send(()).unwrap();
        observed.recv_timeout(Duration::from_secs(1)).unwrap();
        events.send(()).unwrap();
        observed.recv_timeout(Duration::from_secs(1)).unwrap();
        drop(events);
        worker.join().unwrap();
    }
}

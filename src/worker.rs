use std::{
    sync::mpsc::{Receiver, RecvTimeoutError},
    time::Duration,
};

// Block until an event or a configured retry; never poll while idle.
pub fn run(receiver: Receiver<()>, mut action: impl FnMut() -> bool, delays: &[Duration]) {
    let mut retry = None;
    let mut next_delay = 0;
    loop {
        let mut new_event = match retry {
            None => match receiver.recv() {
                Ok(()) => true,
                Err(_) => return,
            },
            Some(timeout) => match receiver.recv_timeout(timeout) {
                Ok(()) => true,
                Err(RecvTimeoutError::Timeout) => false,
                Err(RecvTimeoutError::Disconnected) => return,
            },
        };
        // Coalesce queued events; an event during the command stays pending.
        while receiver.try_recv().is_ok() {
            new_event = true;
        }
        if new_event {
            next_delay = 0;
        }
        if action() {
            retry = None;
        } else {
            retry = delays.get(next_delay).copied();
            if retry.is_some() {
                next_delay += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::mpsc, thread};

    #[test]
    fn no_retries_waits_for_another_event() {
        let (events, receiver) = mpsc::sync_channel(1);
        let (calls, observed) = mpsc::channel();
        let worker = thread::spawn(move || {
            run(
                receiver,
                || {
                    calls.send(()).unwrap();
                    false
                },
                &[],
            );
        });
        events.send(()).unwrap();
        observed.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(observed.recv_timeout(Duration::from_millis(50)).is_err());
        events.send(()).unwrap();
        observed.recv_timeout(Duration::from_secs(1)).unwrap();
        drop(events);
        worker.join().unwrap();
    }

    #[test]
    fn exhausted_budget_waits_for_a_new_event() {
        let (events, receiver) = mpsc::sync_channel(1);
        let (calls, observed) = mpsc::channel();
        let worker = thread::spawn(move || {
            run(
                receiver,
                || {
                    calls.send(()).unwrap();
                    false
                },
                &[Duration::from_millis(5), Duration::from_millis(15)],
            );
        });
        for _ in 0..2 {
            events.send(()).unwrap();
            for _ in 0..3 {
                observed.recv_timeout(Duration::from_secs(1)).unwrap();
            }
            assert!(observed.recv_timeout(Duration::from_millis(50)).is_err());
        }
        drop(events);
        worker.join().unwrap();
    }

    #[test]
    fn success_cancels_remaining_retries() {
        let (events, receiver) = mpsc::sync_channel(1);
        let (calls, observed) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut count = 0;
            run(
                receiver,
                || {
                    count += 1;
                    calls.send(count).unwrap();
                    count == 2
                },
                &[Duration::from_millis(5), Duration::from_millis(15)],
            );
        });
        events.send(()).unwrap();
        assert_eq!(observed.recv_timeout(Duration::from_secs(1)).unwrap(), 1);
        assert_eq!(observed.recv_timeout(Duration::from_secs(1)).unwrap(), 2);
        assert!(observed.recv_timeout(Duration::from_millis(50)).is_err());
        drop(events);
        worker.join().unwrap();
    }

    #[test]
    fn new_event_interrupts_and_resets_retry_schedule() {
        let (events, receiver) = mpsc::sync_channel(1);
        let (calls, observed) = mpsc::channel();
        let worker = thread::spawn(move || {
            run(
                receiver,
                || {
                    calls.send(()).unwrap();
                    false
                },
                &[Duration::from_millis(5), Duration::from_secs(60)],
            );
        });
        events.send(()).unwrap();
        observed.recv_timeout(Duration::from_secs(1)).unwrap();
        observed.recv_timeout(Duration::from_secs(1)).unwrap();
        events.send(()).unwrap();
        observed.recv_timeout(Duration::from_secs(1)).unwrap();
        observed.recv_timeout(Duration::from_secs(1)).unwrap();
        drop(events);
        worker.join().unwrap();
    }
}

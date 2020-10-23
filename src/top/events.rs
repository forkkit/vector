use crossterm::event::{poll, read, Event, KeyCode};
use tokio::sync::{mpsc, oneshot};

static INPUT_INVARIANT: &str = "Couldn't capture keyboard input. Please report.";

/// Capture keyboard input, and send it upstream via a channel. This is used for interaction
/// with the dashboard, and exiting from `vector top`.
pub fn capture_key_press() -> (mpsc::Receiver<KeyCode>, oneshot::Sender<()>) {
    let (tx, rx) = mpsc::channel(5);
    let (kill_tx, mut kill_rx) = oneshot::channel();

    tokio::spawn(async move {
        loop {
            if poll(std::time::Duration::from_millis(50)).unwrap_or(false) {
                match read().expect(INPUT_INVARIANT) {
                    Event::Key(k) => {
                        let _ = tx.clone().send(k.code).await;
                    }
                    _ => {}
                };
            } else if let Ok(_) = kill_rx.try_recv() {
                return;
            }
        }
    });

    (rx, kill_tx)
}

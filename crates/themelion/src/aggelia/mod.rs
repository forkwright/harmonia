pub mod events;

pub use events::HarmoniaEvent;

use tokio::sync::broadcast;

pub type EventSender = broadcast::Sender<HarmoniaEvent>;
pub type EventReceiver = broadcast::Receiver<HarmoniaEvent>;

pub fn create_event_bus(buffer_size: usize) -> (EventSender, EventReceiver) {
    broadcast::channel(buffer_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_event_bus_send_receive() {
        let (tx, mut rx) = create_event_bus(32);
        use crate::ids::MediaId;
        use crate::media::MediaType;
        use std::path::PathBuf;

        tx.send(HarmoniaEvent::ImportCompleted {
            media_id: MediaId::new(),
            media_type: MediaType::Podcast,
            path: PathBuf::from("/podcasts/ep1.mp3"),
        })
        .unwrap();

        let event = rx.recv().await.unwrap();
        assert!(matches!(event, HarmoniaEvent::ImportCompleted { .. }));
    }

    #[tokio::test]
    async fn multiple_subscribers_each_receive_event() {
        let (tx, mut rx1) = create_event_bus(32);
        let mut rx2 = tx.subscribe();

        tx.send(HarmoniaEvent::LibraryScanCompleted {
            items_scanned: 100,
            items_added: 10,
            items_removed: 2,
        })
        .unwrap();

        assert!(matches!(
            rx1.recv().await.unwrap(),
            HarmoniaEvent::LibraryScanCompleted { .. }
        ));
        assert!(matches!(
            rx2.recv().await.unwrap(),
            HarmoniaEvent::LibraryScanCompleted { .. }
        ));
    }
}

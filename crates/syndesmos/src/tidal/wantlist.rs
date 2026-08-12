//! Tidal want-list sync — fetches favorites and emits TidalWantListSynced.

use std::collections::HashSet;

use aggelmata::{EventSender, HarmoniaEvent, MediaId};
use tracing::instrument;

use crate::error::SyndesmodError;
use crate::retry::{CircuitBreaker, with_retry};
use crate::tidal::{TidalApi, TidalFavorite, TidalId};

// WHY: matches the `wants.source` CHECK constraint value for Tidal-sourced
// rows (apotheke migration 001) — the read-side diff must use the same
// source tag the want-insertion path writes.
pub(crate) const TIDAL_WANT_SOURCE: &str = "tidal_sync";

// WHY: a favorite newly detected during a sync — the generated `MediaId`
// paired with its source `TidalFavorite`, so the caller can both emit the
// `MediaId` and persist a want row keyed on the Tidal ID.
#[derive(Debug, Clone)]
pub(crate) struct NewFavorite {
    pub(crate) media_id: MediaId,
    pub(crate) favorite: TidalFavorite,
}

/// Syncs Tidal favorites against known want-list entries.
///
/// Returns the newly detected favorites (each carrying a generated `MediaId`
/// and its source `TidalFavorite`), and emits `TidalWantListSynced` on the
/// event bus for the monitoring layer to consume.
///
/// `existing_tidal_ids` should contain all Tidal IDs already in the want list
/// so the sync can compute the delta. Persisting the returned favorites as
/// wants is the caller's responsibility (it owns the DB handle).
#[instrument(skip(api, event_tx, circuit, existing_tidal_ids))]
pub(crate) async fn sync_want_list(
    api: &dyn TidalApi,
    event_tx: &EventSender,
    existing_tidal_ids: &HashSet<TidalId>,
    circuit: &CircuitBreaker,
) -> Result<Vec<NewFavorite>, SyndesmodError> {
    let favorites: Vec<TidalFavorite> = with_retry(|| api.fetch_favorites(), circuit).await?;

    let new: Vec<NewFavorite> = favorites
        .into_iter()
        .filter(|fav| !existing_tidal_ids.contains(&fav.tidal_id))
        .map(|favorite| NewFavorite {
            media_id: MediaId::new(),
            favorite,
        })
        .collect();

    if !new.is_empty() {
        let added: Vec<MediaId> = new.iter().map(|nf| nf.media_id).collect();
        event_tx
            .send(HarmoniaEvent::TidalWantListSynced { added })
            .ok();
    }

    Ok(new)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use aggelmata::create_event_bus;

    use super::*;
    use crate::retry::CircuitBreaker;
    use crate::tidal::tests::MockTidalApi;
    use crate::tidal::{TidalFavorite, TidalId};

    fn breaker() -> CircuitBreaker {
        CircuitBreaker::new("tidal", 5, std::time::Duration::from_secs(300))
    }

    fn make_favorite(id: &str) -> TidalFavorite {
        TidalFavorite {
            tidal_id: TidalId(id.to_string()),
            title: format!("Track {}", id),
            artist: "Test Artist".to_string(),
        }
    }

    #[tokio::test]
    async fn returns_new_favorites_not_in_existing_set() {
        let (tx, _rx) = create_event_bus(32);
        let favorites = vec![
            make_favorite("t1"),
            make_favorite("t2"),
            make_favorite("t3"),
        ];
        let mock = MockTidalApi::new(favorites);
        let circuit = breaker();

        let mut existing = HashSet::new();
        existing.insert(TidalId("t1".to_string()));

        let added = sync_want_list(&mock, &tx, &existing, &circuit)
            .await
            .unwrap();

        assert_eq!(added.len(), 2);
    }

    #[tokio::test]
    async fn emits_tidal_want_list_synced_event_for_new_favorites() {
        let (tx, mut rx) = create_event_bus(32);
        let favorites = vec![make_favorite("t10"), make_favorite("t20")];
        let mock = MockTidalApi::new(favorites);
        let circuit = breaker();

        let existing = HashSet::new();
        let added = sync_want_list(&mock, &tx, &existing, &circuit)
            .await
            .unwrap();

        assert_eq!(added.len(), 2);

        let event = rx.recv().await.unwrap();
        assert!(matches!(
            event,
            HarmoniaEvent::TidalWantListSynced { added } if added.len() == 2
        ));
    }

    #[tokio::test]
    async fn emits_no_event_when_no_new_favorites() {
        let (tx, mut rx) = create_event_bus(32);
        let favorites = vec![make_favorite("t1")];
        let mock = MockTidalApi::new(favorites);
        let circuit = breaker();

        let mut existing = HashSet::new();
        existing.insert(TidalId("t1".to_string()));

        let added = sync_want_list(&mock, &tx, &existing, &circuit)
            .await
            .unwrap();

        assert!(added.is_empty());
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn empty_favorites_list_returns_empty() {
        let (tx, _rx) = create_event_bus(32);
        let mock = MockTidalApi::new(vec![]);
        let circuit = breaker();
        let existing = HashSet::new();

        let added = sync_want_list(&mock, &tx, &existing, &circuit)
            .await
            .unwrap();

        assert!(added.is_empty());
    }

    #[tokio::test]
    async fn parse_favorites_extracts_tidal_id_and_title() {
        let body = serde_json::json!({
            "data": [
                {
                    "resource": {
                        "id": "123456",
                        "title": "Roygbiv",
                        "artists": [{ "name": "Boards of Canada" }]
                    }
                }
            ]
        });
        let favorites = crate::tidal::parse_favorites(&body);
        assert_eq!(favorites.len(), 1);
        assert_eq!(favorites[0].tidal_id, TidalId("123456".to_string()));
        assert_eq!(favorites[0].title, "Roygbiv");
        assert_eq!(favorites[0].artist, "Boards of Canada");
    }
}

macro_rules! define_id {
    ($name:ident, $display_prefix:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct $name(uuid::Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(uuid::Uuid::now_v7())
            }

            pub fn from_uuid(id: uuid::Uuid) -> Self {
                Self(id)
            }

            pub fn as_uuid(&self) -> &uuid::Uuid {
                &self.0
            }

            pub fn into_uuid(self) -> uuid::Uuid {
                self.0
            }

            pub fn as_bytes(&self) -> &[u8; 16] {
                self.0.as_bytes()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}{}", $display_prefix, self.0)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

define_id!(MediaId, "med-");
define_id!(UserId, "usr-");
define_id!(DownloadId, "dl-");
define_id!(WantId, "want-");
define_id!(ReleaseId, "rel-");
define_id!(HaveId, "have-");
define_id!(ApiKeyId, "key-");
define_id!(RegistryId, "reg-");
define_id!(FeedId, "feed-");
define_id!(EpisodeId, "ep-");
define_id!(RequestId, "req-");
define_id!(QueryId, "qry-");

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn new_creates_unique_ids() {
        let a = MediaId::new();
        let b = MediaId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn display_includes_prefix() {
        let id = MediaId::new();
        assert!(id.to_string().starts_with("med-"));

        let id = UserId::new();
        assert!(id.to_string().starts_with("usr-"));

        let id = DownloadId::new();
        assert!(id.to_string().starts_with("dl-"));

        let id = WantId::new();
        assert!(id.to_string().starts_with("want-"));

        let id = ReleaseId::new();
        assert!(id.to_string().starts_with("rel-"));

        let id = HaveId::new();
        assert!(id.to_string().starts_with("have-"));

        let id = ApiKeyId::new();
        assert!(id.to_string().starts_with("key-"));

        let id = RegistryId::new();
        assert!(id.to_string().starts_with("reg-"));

        let id = FeedId::new();
        assert!(id.to_string().starts_with("feed-"));

        let id = EpisodeId::new();
        assert!(id.to_string().starts_with("ep-"));

        let id = RequestId::new();
        assert!(id.to_string().starts_with("req-"));

        let id = QueryId::new();
        assert!(id.to_string().starts_with("qry-"));
    }

    #[test]
    fn serde_roundtrip() {
        let original = MediaId::new();
        let json = serde_json::to_string(&original).unwrap();
        let recovered: MediaId = serde_json::from_str(&json).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn from_uuid_roundtrip() {
        let uuid = uuid::Uuid::now_v7();
        let id = MediaId::from_uuid(uuid);
        assert_eq!(id.into_uuid(), uuid);
    }

    #[test]
    fn default_creates_valid_id() {
        let id = MediaId::default();
        assert!(!id.to_string().is_empty());
    }
}

use crate::Config;

pub struct ConfigChange {
    pub field: String,
    pub requires_restart: bool,
}

const RESTART_REQUIRED: &[&str] = &["database", "exousia"];

/// Compare two configs and return a list of changed top-level sections.
pub fn diff_config(old: &Config, new: &Config) -> Vec<ConfigChange> {
    let old_val = serde_json::to_value(old).unwrap_or_default();
    let new_val = serde_json::to_value(new).unwrap_or_default();

    let mut changes = Vec::new();

    if let (Some(old_map), Some(new_map)) = (old_val.as_object(), new_val.as_object()) {
        let all_keys: std::collections::HashSet<&String> =
            old_map.keys().chain(new_map.keys()).collect();

        for key in all_keys {
            if old_map.get(key) != new_map.get(key) {
                changes.push(ConfigChange {
                    field: key.clone(),
                    requires_restart: RESTART_REQUIRED.contains(&key.as_str()),
                });
            }
        }
    }

    changes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;

    fn base_config() -> Config {
        let mut c = Config::default();
        c.exousia.jwt_secret = "a-very-long-secret-key-that-is-at-least-32-bytes-long".into();
        c
    }

    #[test]
    fn identical_configs_return_no_changes() {
        let c = base_config();
        assert!(diff_config(&c, &c).is_empty());
    }

    #[test]
    fn changed_paroche_returns_non_restart_change() {
        let old = base_config();
        let mut new = base_config();
        new.paroche.port = 9090;

        let changes = diff_config(&old, &new);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field, "paroche");
        assert!(!changes[0].requires_restart);
    }

    #[test]
    fn changed_database_returns_restart_required() {
        let old = base_config();
        let mut new = base_config();
        new.database.db_path = std::path::PathBuf::from("/new/path/harmonia.db");

        let changes = diff_config(&old, &new);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field, "database");
        assert!(changes[0].requires_restart);
    }

    #[test]
    fn multiple_changed_sections_return_multiple_entries() {
        let old = base_config();
        let mut new = base_config();
        new.paroche.port = 9090;
        new.kritike.scan_interval_hours = 12;

        let changes = diff_config(&old, &new);
        assert_eq!(changes.len(), 2);
        let fields: std::collections::HashSet<&str> =
            changes.iter().map(|c| c.field.as_str()).collect();
        assert!(fields.contains("paroche"));
        assert!(fields.contains("kritike"));
        assert!(changes.iter().all(|c| !c.requires_restart));
    }
}

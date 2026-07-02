//! Torznab category table and site-category mapping for Cardigann definitions.

use tracing::debug;

use crate::client::cardigann::definition::CategoryMapping;
use crate::types::IndexerCategory;

/// The standard Torznab/Newznab category tree, as named by Cardigann
/// definitions in `caps.categorymappings[].cat`.
const TORZNAB_CATEGORIES: &[(u32, &str)] = &[
    (1000, "Console"),
    (1010, "Console/NDS"),
    (1020, "Console/PSP"),
    (1030, "Console/Wii"),
    (1040, "Console/XBox"),
    (1050, "Console/XBox 360"),
    (1060, "Console/Wiiware"),
    (1070, "Console/XBox 360 DLC"),
    (1080, "Console/PS3"),
    (1090, "Console/Other"),
    (1110, "Console/3DS"),
    (1120, "Console/PS Vita"),
    (1130, "Console/WiiU"),
    (1140, "Console/XBox One"),
    (1180, "Console/PS4"),
    (2000, "Movies"),
    (2010, "Movies/Foreign"),
    (2020, "Movies/Other"),
    (2030, "Movies/SD"),
    (2040, "Movies/HD"),
    (2045, "Movies/UHD"),
    (2050, "Movies/BluRay"),
    (2060, "Movies/3D"),
    (2070, "Movies/DVD"),
    (2080, "Movies/WEB-DL"),
    (3000, "Audio"),
    (3010, "Audio/MP3"),
    (3020, "Audio/Video"),
    (3030, "Audio/Audiobook"),
    (3040, "Audio/Lossless"),
    (3050, "Audio/Other"),
    (3060, "Audio/Foreign"),
    (4000, "PC"),
    (4010, "PC/0day"),
    (4020, "PC/ISO"),
    (4030, "PC/Mac"),
    (4040, "PC/Mobile-Other"),
    (4050, "PC/Games"),
    (4060, "PC/Mobile-iOS"),
    (4070, "PC/Mobile-Android"),
    (5000, "TV"),
    (5010, "TV/WEB-DL"),
    (5020, "TV/Foreign"),
    (5030, "TV/SD"),
    (5040, "TV/HD"),
    (5045, "TV/UHD"),
    (5050, "TV/Other"),
    (5060, "TV/Sport"),
    (5070, "TV/Anime"),
    (5080, "TV/Documentary"),
    (6000, "XXX"),
    (6010, "XXX/DVD"),
    (6020, "XXX/WMV"),
    (6030, "XXX/XviD"),
    (6040, "XXX/x264"),
    (6045, "XXX/UHD"),
    (6050, "XXX/Pack"),
    (6060, "XXX/ImageSet"),
    (6070, "XXX/Other"),
    (6080, "XXX/SD"),
    (6090, "XXX/WEB-DL"),
    (7000, "Books"),
    (7010, "Books/Mags"),
    (7020, "Books/EBook"),
    (7030, "Books/Comics"),
    (7040, "Books/Technical"),
    (7050, "Books/Other"),
    (7060, "Books/Foreign"),
    (8000, "Other"),
    (8010, "Other/Misc"),
    (8020, "Other/Hashed"),
];

pub fn category_id_for_name(name: &str) -> Option<u32> {
    TORZNAB_CATEGORIES
        .iter()
        .find(|(_, n)| n.eq_ignore_ascii_case(name.trim()))
        .map(|(id, _)| *id)
}

pub fn category_name_for_id(id: u32) -> Option<&'static str> {
    TORZNAB_CATEGORIES
        .iter()
        .find(|(i, _)| *i == id)
        .map(|(_, n)| *n)
}

/// Maps the query's Torznab category ids to the site-native category ids the
/// definition declares. A top-level request (e.g. 2000) also selects every
/// mapping in its subtree (2040, 2045, ...).
pub fn site_categories_for(mappings: &[CategoryMapping], requested: &[u32]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for mapping in mappings {
        let Some(torznab_id) = category_id_for_name(&mapping.cat) else {
            continue;
        };
        let selected = requested
            .iter()
            .any(|r| torznab_id == *r || (*r % 1000 == 0 && torznab_id / 1000 == *r / 1000));
        if selected && !out.contains(&mapping.id.0) {
            out.push(mapping.id.0.clone());
        }
    }
    out
}

/// Maps one extracted site category id back to its Torznab category id.
pub fn torznab_id_for_site(mappings: &[CategoryMapping], site_id: &str) -> Option<u32> {
    let site_id = site_id.trim();
    let mapped = mappings
        .iter()
        .find(|m| m.id.0 == site_id)
        .and_then(|m| category_id_for_name(&m.cat));
    if mapped.is_none() {
        debug!(site_id = %site_id, "site category has no Torznab mapping");
    }
    mapped
}

/// The Torznab categories a definition serves, deduplicated and ordered by
/// id, for `caps()`. Unknown category names are skipped.
pub fn caps_categories(mappings: &[CategoryMapping]) -> Vec<IndexerCategory> {
    let mut ids: Vec<u32> = mappings
        .iter()
        .filter_map(|m| {
            let id = category_id_for_name(&m.cat);
            if id.is_none() {
                debug!(cat = %m.cat, "unknown Torznab category name in definition; skipped");
            }
            id
        })
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids.into_iter()
        .filter_map(|id| {
            category_name_for_id(id).map(|name| IndexerCategory {
                id,
                name: name.to_string(),
                subcategories: Vec::new(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::cardigann::definition::ScalarString;

    fn mapping(id: &str, cat: &str) -> CategoryMapping {
        CategoryMapping {
            id: ScalarString(id.to_string()),
            cat: cat.to_string(),
            desc: None,
            default: false,
        }
    }

    #[test]
    fn name_lookup_is_case_insensitive() {
        assert_eq!(category_id_for_name("Movies/HD"), Some(2040));
        assert_eq!(category_id_for_name("movies/hd"), Some(2040));
        assert_eq!(category_id_for_name("Nonsense"), None);
    }

    #[test]
    fn exact_category_selects_site_id() {
        let mappings = [mapping("6", "Movies/HD"), mapping("7", "TV/HD")];
        assert_eq!(site_categories_for(&mappings, &[2040]), vec!["6"]);
    }

    #[test]
    fn top_level_request_selects_subtree() {
        let mappings = [
            mapping("6", "Movies/HD"),
            mapping("8", "Movies/SD"),
            mapping("7", "TV/HD"),
        ];
        assert_eq!(site_categories_for(&mappings, &[2000]), vec!["6", "8"]);
    }

    #[test]
    fn duplicate_site_ids_are_deduplicated() {
        let mappings = [mapping("6", "Movies/HD"), mapping("6", "Movies/SD")];
        assert_eq!(site_categories_for(&mappings, &[2000]), vec!["6"]);
    }

    #[test]
    fn site_id_maps_back_to_torznab() {
        let mappings = [mapping("6", "Movies/HD")];
        assert_eq!(torznab_id_for_site(&mappings, "6"), Some(2040));
        assert_eq!(torznab_id_for_site(&mappings, " 6 "), Some(2040));
        assert_eq!(torznab_id_for_site(&mappings, "99"), None);
    }

    #[test]
    fn caps_categories_dedupes_and_sorts() {
        let mappings = [
            mapping("7", "TV/HD"),
            mapping("6", "Movies/HD"),
            mapping("9", "Movies/HD"),
            mapping("x", "Nonsense"),
        ];
        let caps = caps_categories(&mappings);
        let pairs: Vec<(u32, &str)> = caps.iter().map(|c| (c.id, c.name.as_str())).collect();
        assert_eq!(pairs, vec![(2040, "Movies/HD"), (5040, "TV/HD")]);
    }
}

use serde::Serialize;

pub const MIME_OPDS_V2: &str = "application/opds+json";

#[derive(Serialize)]
pub struct OpdsFeed {
    pub metadata: FeedMetadata,
    pub links: Vec<OpdsLink>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub navigation: Vec<NavigationLink>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub publications: Vec<Publication>,
}

#[derive(Serialize)]
pub struct FeedMetadata {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "numberOfItems")]
    pub number_of_items: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "itemsPerPage")]
    pub items_per_page: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "currentPage")]
    pub current_page: Option<u64>,
}

#[derive(Serialize, Clone)]
pub struct OpdsLink {
    pub rel: String,
    pub href: String,
    #[serde(rename = "type")]
    pub link_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub templated: Option<bool>,
}

impl OpdsLink {
    pub fn new(
        rel: impl Into<String>,
        href: impl Into<String>,
        link_type: impl Into<String>,
    ) -> Self {
        Self {
            rel: rel.into(),
            href: href.into(),
            link_type: link_type.into(),
            title: None,
            templated: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn as_template(mut self) -> Self {
        self.templated = Some(true);
        self
    }
}

#[derive(Serialize)]
pub struct NavigationLink {
    pub href: String,
    pub title: String,
    #[serde(rename = "type")]
    pub link_type: String,
    pub rel: String,
}

#[derive(Serialize)]
pub struct Publication {
    pub metadata: PublicationMetadata,
    pub links: Vec<OpdsLink>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<OpdsLink>,
}

#[derive(Serialize)]
pub struct PublicationMetadata {
    #[serde(rename = "@type")]
    pub pub_type: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Vec<Contributor>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Serialize)]
pub struct Contributor {
    pub name: String,
}

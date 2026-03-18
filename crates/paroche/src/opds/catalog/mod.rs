use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{Response, StatusCode, header},
    response::IntoResponse,
};
use exousia::AuthenticatedUser;
use serde::Deserialize;
use uuid::Uuid;

use crate::{error::ParocheError, routes::music::chrono_now_pub, state::AppState};

use super::{
    acquisition,
    types_v1::{AtomEntry, AtomFeed, AtomLink, MIME_OPDS_V1, MIME_OPENSEARCH},
    types_v2::{
        Contributor, FeedMetadata, MIME_OPDS_V2, NavigationLink, OpdsFeed, OpdsLink, Publication,
        PublicationMetadata,
    },
};

fn bytes_to_uuid_str(bytes: &[u8]) -> String {
    Uuid::from_slice(bytes)
        .map(|u| u.to_string())
        .unwrap_or_default()
}

pub struct OpdsV2Response(pub OpdsFeed);

impl IntoResponse for OpdsV2Response {
    fn into_response(self) -> Response<Body> {
        match serde_json::to_vec(&self.0) {
            Ok(json) => Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, MIME_OPDS_V2)
                .body(Body::from(json))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub struct OpdsV1Response(pub String);

impl IntoResponse for OpdsV1Response {
    fn into_response(self) -> Response<Body> {
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, MIME_OPDS_V1)
            .body(Body::from(self.0))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
    }
}

pub struct OpdsOpenSearchResponse(pub String);

impl IntoResponse for OpdsOpenSearchResponse {
    fn into_response(self) -> Response<Body> {
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, MIME_OPENSEARCH)
            .body(Body::from(self.0))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
    }
}

#[derive(Deserialize)]
pub struct OpdsPageQuery {
    #[serde(default = "default_page")]
    pub page: u64,
}

fn default_page() -> u64 {
    1
}

pub fn book_to_publication(book: &harmonia_db::repo::book::Book) -> Publication {
    let id_str = bytes_to_uuid_str(&book.id);
    let mime = acquisition::effective_mime(book.file_format.as_deref(), book.file_path.as_deref());

    let author = book
        .publisher
        .as_ref()
        .map(|p| vec![Contributor { name: p.clone() }]);

    Publication {
        metadata: PublicationMetadata {
            pub_type: "http://schema.org/Book".to_string(),
            title: book.title.clone(),
            author,
            publisher: book.publisher.clone(),
            description: book.description.clone(),
            identifier: book.isbn13.clone().or_else(|| book.isbn.clone()),
            language: book.language.clone(),
        },
        links: vec![OpdsLink::new(
            "http://opds-spec.org/acquisition",
            format!("/api/books/{id_str}/download"),
            mime,
        )],
        images: vec![
            OpdsLink::new(
                "http://opds-spec.org/image",
                format!("/api/books/{id_str}/cover"),
                "image/jpeg",
            ),
            OpdsLink::new(
                "http://opds-spec.org/image/thumbnail",
                format!("/api/books/{id_str}/cover?size=thumbnail"),
                "image/jpeg",
            ),
        ],
    }
}

pub fn comic_to_publication(comic: &harmonia_db::repo::comic::Comic) -> Publication {
    let id_str = bytes_to_uuid_str(&comic.id);
    let mime =
        acquisition::effective_mime(comic.file_format.as_deref(), comic.file_path.as_deref());

    let author = comic
        .comicinfo_writer
        .as_ref()
        .map(|w| vec![Contributor { name: w.clone() }]);

    let title = match &comic.title {
        Some(t) => format!("{} — {t}", comic.series_name),
        None => comic.series_name.clone(),
    };

    Publication {
        metadata: PublicationMetadata {
            pub_type: "http://schema.org/ComicStory".to_string(),
            title,
            author,
            publisher: comic.publisher.clone(),
            description: comic.summary.clone(),
            identifier: None,
            language: comic.language.clone(),
        },
        links: vec![OpdsLink::new(
            "http://opds-spec.org/acquisition",
            format!("/api/comics/{id_str}/download"),
            mime,
        )],
        images: vec![
            OpdsLink::new(
                "http://opds-spec.org/image",
                format!("/api/comics/{id_str}/cover"),
                "image/jpeg",
            ),
            OpdsLink::new(
                "http://opds-spec.org/image/thumbnail",
                format!("/api/comics/{id_str}/cover?size=thumbnail"),
                "image/jpeg",
            ),
        ],
    }
}

pub fn book_to_atom_entry(book: &harmonia_db::repo::book::Book) -> AtomEntry {
    let id_str = bytes_to_uuid_str(&book.id);
    let mime = acquisition::effective_mime(book.file_format.as_deref(), book.file_path.as_deref());
    AtomEntry {
        id: format!("urn:harmonia:book:{id_str}"),
        title: book.title.clone(),
        updated: book.added_at.clone(),
        summary: book.description.clone(),
        links: vec![
            AtomLink {
                rel: "http://opds-spec.org/acquisition".to_string(),
                href: format!("/api/books/{id_str}/download"),
                link_type: mime.to_string(),
                title: None,
            },
            AtomLink {
                rel: "http://opds-spec.org/image".to_string(),
                href: format!("/api/books/{id_str}/cover"),
                link_type: "image/jpeg".to_string(),
                title: None,
            },
        ],
    }
}

pub fn comic_to_atom_entry(comic: &harmonia_db::repo::comic::Comic) -> AtomEntry {
    let id_str = bytes_to_uuid_str(&comic.id);
    let mime =
        acquisition::effective_mime(comic.file_format.as_deref(), comic.file_path.as_deref());
    let title = match &comic.title {
        Some(t) => format!("{} — {t}", comic.series_name),
        None => comic.series_name.clone(),
    };
    AtomEntry {
        id: format!("urn:harmonia:comic:{id_str}"),
        title,
        updated: comic.added_at.clone(),
        summary: comic.summary.clone(),
        links: vec![
            AtomLink {
                rel: "http://opds-spec.org/acquisition".to_string(),
                href: format!("/api/comics/{id_str}/download"),
                link_type: mime.to_string(),
                title: None,
            },
            AtomLink {
                rel: "http://opds-spec.org/image".to_string(),
                href: format!("/api/comics/{id_str}/cover"),
                link_type: "image/jpeg".to_string(),
                title: None,
            },
        ],
    }
}

pub async fn catalog_v2(
    State(_state): State<AppState>,
    _auth: AuthenticatedUser,
) -> Result<OpdsV2Response, ParocheError> {
    let feed = OpdsFeed {
        metadata: FeedMetadata {
            title: "Harmonia Library".to_string(),
            number_of_items: None,
            items_per_page: None,
            current_page: None,
        },
        links: vec![
            OpdsLink::new("self", "/opds/v2/catalog", MIME_OPDS_V2),
            OpdsLink::new("search", "/opds/v2/search?q={searchTerms}", MIME_OPDS_V2).as_template(),
        ],
        navigation: vec![
            NavigationLink {
                href: "/opds/v2/shelf/new-arrivals".to_string(),
                title: "New Arrivals".to_string(),
                link_type: MIME_OPDS_V2.to_string(),
                rel: "http://opds-spec.org/sort/new".to_string(),
            },
            NavigationLink {
                href: "/opds/v2/books".to_string(),
                title: "All Books".to_string(),
                link_type: MIME_OPDS_V2.to_string(),
                rel: "subsection".to_string(),
            },
            NavigationLink {
                href: "/opds/v2/comics".to_string(),
                title: "All Comics".to_string(),
                link_type: MIME_OPDS_V2.to_string(),
                rel: "subsection".to_string(),
            },
            NavigationLink {
                href: "/opds/v2/shelf/authors".to_string(),
                title: "Authors".to_string(),
                link_type: MIME_OPDS_V2.to_string(),
                rel: "subsection".to_string(),
            },
            NavigationLink {
                href: "/opds/v2/shelf/series".to_string(),
                title: "Series".to_string(),
                link_type: MIME_OPDS_V2.to_string(),
                rel: "subsection".to_string(),
            },
        ],
        publications: vec![],
    };
    Ok(OpdsV2Response(feed))
}

pub async fn books_v2(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(pq): Query<OpdsPageQuery>,
) -> Result<OpdsV2Response, ParocheError> {
    let page = pq.page.max(1);
    let page_size = state.config.paroche.opds_page_size as i64;
    let offset = ((page - 1) * page_size as u64) as i64;

    let mut books =
        harmonia_db::repo::book::list_books(&state.db.read, page_size + 1, offset).await?;

    let has_next = books.len() > page_size as usize;
    books.truncate(page_size as usize);

    let mut links = vec![
        OpdsLink::new("self", format!("/opds/v2/books?page={page}"), MIME_OPDS_V2),
        OpdsLink::new("start", "/opds/v2/catalog", MIME_OPDS_V2),
    ];
    if has_next {
        links.push(OpdsLink::new(
            "next",
            format!("/opds/v2/books?page={}", page + 1),
            MIME_OPDS_V2,
        ));
    }

    let count = books.len() as u64;
    let publications: Vec<_> = books.iter().map(book_to_publication).collect();

    Ok(OpdsV2Response(OpdsFeed {
        metadata: FeedMetadata {
            title: "All Books".to_string(),
            number_of_items: Some(count),
            items_per_page: Some(page_size as u64),
            current_page: Some(page),
        },
        links,
        navigation: vec![],
        publications,
    }))
}

pub async fn comics_v2(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(pq): Query<OpdsPageQuery>,
) -> Result<OpdsV2Response, ParocheError> {
    let page = pq.page.max(1);
    let page_size = state.config.paroche.opds_page_size as i64;
    let offset = ((page - 1) * page_size as u64) as i64;

    let mut comics =
        harmonia_db::repo::comic::list_comics(&state.db.read, page_size + 1, offset).await?;

    let has_next = comics.len() > page_size as usize;
    comics.truncate(page_size as usize);

    let mut links = vec![
        OpdsLink::new("self", format!("/opds/v2/comics?page={page}"), MIME_OPDS_V2),
        OpdsLink::new("start", "/opds/v2/catalog", MIME_OPDS_V2),
    ];
    if has_next {
        links.push(OpdsLink::new(
            "next",
            format!("/opds/v2/comics?page={}", page + 1),
            MIME_OPDS_V2,
        ));
    }

    let count = comics.len() as u64;
    let publications: Vec<_> = comics.iter().map(comic_to_publication).collect();

    Ok(OpdsV2Response(OpdsFeed {
        metadata: FeedMetadata {
            title: "All Comics".to_string(),
            number_of_items: Some(count),
            items_per_page: Some(page_size as u64),
            current_page: Some(page),
        },
        links,
        navigation: vec![],
        publications,
    }))
}

pub async fn book_v2(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<OpdsV2Response, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    let book = harmonia_db::repo::book::get_book(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    let publication = book_to_publication(&book);

    Ok(OpdsV2Response(OpdsFeed {
        metadata: FeedMetadata {
            title: book.title.clone(),
            number_of_items: Some(1),
            items_per_page: None,
            current_page: None,
        },
        links: vec![OpdsLink::new(
            "self",
            format!("/opds/v2/books/{id}"),
            MIME_OPDS_V2,
        )],
        navigation: vec![],
        publications: vec![publication],
    }))
}

pub async fn comic_v2(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<OpdsV2Response, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    let comic = harmonia_db::repo::comic::get_comic(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    let publication = comic_to_publication(&comic);

    Ok(OpdsV2Response(OpdsFeed {
        metadata: FeedMetadata {
            title: comic.series_name.clone(),
            number_of_items: Some(1),
            items_per_page: None,
            current_page: None,
        },
        links: vec![OpdsLink::new(
            "self",
            format!("/opds/v2/comics/{id}"),
            MIME_OPDS_V2,
        )],
        navigation: vec![],
        publications: vec![publication],
    }))
}

pub async fn shelf_v2(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(shelf): Path<String>,
    Query(pq): Query<OpdsPageQuery>,
) -> Result<OpdsV2Response, ParocheError> {
    match shelf.as_str() {
        "new-arrivals" => {
            let page = pq.page.max(1);
            let page_size = state.config.paroche.opds_page_size as i64;
            let offset = ((page - 1) * page_size as u64) as i64;

            let mut books =
                harmonia_db::repo::book::list_books(&state.db.read, page_size + 1, offset).await?;
            let has_next = books.len() > page_size as usize;
            books.truncate(page_size as usize);

            let mut links = vec![
                OpdsLink::new(
                    "self",
                    format!("/opds/v2/shelf/new-arrivals?page={page}"),
                    MIME_OPDS_V2,
                ),
                OpdsLink::new("start", "/opds/v2/catalog", MIME_OPDS_V2),
            ];
            if has_next {
                links.push(OpdsLink::new(
                    "next",
                    format!("/opds/v2/shelf/new-arrivals?page={}", page + 1),
                    MIME_OPDS_V2,
                ));
            }

            let count = books.len() as u64;
            let publications: Vec<_> = books.iter().map(book_to_publication).collect();

            Ok(OpdsV2Response(OpdsFeed {
                metadata: FeedMetadata {
                    title: "New Arrivals".to_string(),
                    number_of_items: Some(count),
                    items_per_page: Some(page_size as u64),
                    current_page: Some(page),
                },
                links,
                navigation: vec![],
                publications,
            }))
        }
        "series" => {
            let page = pq.page.max(1);
            let page_size = state.config.paroche.opds_page_size as i64;
            let offset = ((page - 1) * page_size as u64) as i64;

            let mut comics =
                harmonia_db::repo::comic::list_comics(&state.db.read, page_size + 1, offset)
                    .await?;
            let has_next = comics.len() > page_size as usize;
            comics.truncate(page_size as usize);

            let mut links = vec![
                OpdsLink::new(
                    "self",
                    format!("/opds/v2/shelf/series?page={page}"),
                    MIME_OPDS_V2,
                ),
                OpdsLink::new("start", "/opds/v2/catalog", MIME_OPDS_V2),
            ];
            if has_next {
                links.push(OpdsLink::new(
                    "next",
                    format!("/opds/v2/shelf/series?page={}", page + 1),
                    MIME_OPDS_V2,
                ));
            }

            let count = comics.len() as u64;
            let publications: Vec<_> = comics.iter().map(comic_to_publication).collect();

            Ok(OpdsV2Response(OpdsFeed {
                metadata: FeedMetadata {
                    title: "Series".to_string(),
                    number_of_items: Some(count),
                    items_per_page: Some(page_size as u64),
                    current_page: Some(page),
                },
                links,
                navigation: vec![],
                publications,
            }))
        }
        _ => Err(ParocheError::NotFound),
    }
}

pub async fn catalog_v1(
    State(_state): State<AppState>,
    _auth: AuthenticatedUser,
) -> Result<OpdsV1Response, ParocheError> {
    let now = chrono_now_pub();
    let feed = AtomFeed {
        id: "urn:harmonia:catalog".to_string(),
        title: "Harmonia Library".to_string(),
        updated: now.clone(),
        links: vec![
            AtomLink {
                rel: "self".to_string(),
                href: "/opds/v1/catalog.xml".to_string(),
                link_type: "application/atom+xml;profile=opds-catalog".to_string(),
                title: None,
            },
            AtomLink {
                rel: "start".to_string(),
                href: "/opds/v1/catalog.xml".to_string(),
                link_type: "application/atom+xml;profile=opds-catalog".to_string(),
                title: None,
            },
            AtomLink {
                rel: "search".to_string(),
                href: "/opds/v1/search.xml".to_string(),
                link_type: MIME_OPENSEARCH.to_string(),
                title: Some("Search".to_string()),
            },
        ],
        entries: vec![
            AtomEntry {
                id: "urn:harmonia:books".to_string(),
                title: "All Books".to_string(),
                updated: now.clone(),
                summary: None,
                links: vec![AtomLink {
                    rel: "subsection".to_string(),
                    href: "/opds/v1/books.xml".to_string(),
                    link_type: "application/atom+xml;profile=opds-catalog".to_string(),
                    title: None,
                }],
            },
            AtomEntry {
                id: "urn:harmonia:comics".to_string(),
                title: "All Comics".to_string(),
                updated: now.clone(),
                summary: None,
                links: vec![AtomLink {
                    rel: "subsection".to_string(),
                    href: "/opds/v1/comics.xml".to_string(),
                    link_type: "application/atom+xml;profile=opds-catalog".to_string(),
                    title: None,
                }],
            },
        ],
    };
    Ok(OpdsV1Response(feed.to_xml()))
}

pub async fn books_v1(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(pq): Query<OpdsPageQuery>,
) -> Result<OpdsV1Response, ParocheError> {
    let page = pq.page.max(1);
    let page_size = state.config.paroche.opds_page_size as i64;
    let offset = ((page - 1) * page_size as u64) as i64;
    let now = chrono_now_pub();

    let mut books =
        harmonia_db::repo::book::list_books(&state.db.read, page_size + 1, offset).await?;
    let has_next = books.len() > page_size as usize;
    books.truncate(page_size as usize);

    let mut links = vec![
        AtomLink {
            rel: "self".to_string(),
            href: format!("/opds/v1/books.xml?page={page}"),
            link_type: "application/atom+xml;profile=opds-catalog".to_string(),
            title: None,
        },
        AtomLink {
            rel: "start".to_string(),
            href: "/opds/v1/catalog.xml".to_string(),
            link_type: "application/atom+xml;profile=opds-catalog".to_string(),
            title: None,
        },
    ];
    if has_next {
        links.push(AtomLink {
            rel: "next".to_string(),
            href: format!("/opds/v1/books.xml?page={}", page + 1),
            link_type: "application/atom+xml;profile=opds-catalog".to_string(),
            title: None,
        });
    }

    let entries: Vec<_> = books.iter().map(book_to_atom_entry).collect();

    let feed = AtomFeed {
        id: format!("urn:harmonia:books:page:{page}"),
        title: "All Books".to_string(),
        updated: now,
        links,
        entries,
    };
    Ok(OpdsV1Response(feed.to_xml()))
}

pub async fn comics_v1(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(pq): Query<OpdsPageQuery>,
) -> Result<OpdsV1Response, ParocheError> {
    let page = pq.page.max(1);
    let page_size = state.config.paroche.opds_page_size as i64;
    let offset = ((page - 1) * page_size as u64) as i64;
    let now = chrono_now_pub();

    let mut comics =
        harmonia_db::repo::comic::list_comics(&state.db.read, page_size + 1, offset).await?;
    let has_next = comics.len() > page_size as usize;
    comics.truncate(page_size as usize);

    let mut links = vec![
        AtomLink {
            rel: "self".to_string(),
            href: format!("/opds/v1/comics.xml?page={page}"),
            link_type: "application/atom+xml;profile=opds-catalog".to_string(),
            title: None,
        },
        AtomLink {
            rel: "start".to_string(),
            href: "/opds/v1/catalog.xml".to_string(),
            link_type: "application/atom+xml;profile=opds-catalog".to_string(),
            title: None,
        },
    ];
    if has_next {
        links.push(AtomLink {
            rel: "next".to_string(),
            href: format!("/opds/v1/comics.xml?page={}", page + 1),
            link_type: "application/atom+xml;profile=opds-catalog".to_string(),
            title: None,
        });
    }

    let entries: Vec<_> = comics.iter().map(comic_to_atom_entry).collect();

    let feed = AtomFeed {
        id: format!("urn:harmonia:comics:page:{page}"),
        title: "All Comics".to_string(),
        updated: now,
        links,
        entries,
    };
    Ok(OpdsV1Response(feed.to_xml()))
}

pub async fn entry_v1(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<OpdsV1Response, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();
    let now = chrono_now_pub();

    if let Some(book) = harmonia_db::repo::book::get_book(&state.db.read, &id_bytes).await? {
        let entry = book_to_atom_entry(&book);
        let feed = AtomFeed {
            id: entry.id.clone(),
            title: entry.title.clone(),
            updated: now,
            links: vec![AtomLink {
                rel: "self".to_string(),
                href: format!("/opds/v1/entry/{id}"),
                link_type: "application/atom+xml;profile=opds-catalog".to_string(),
                title: None,
            }],
            entries: vec![entry],
        };
        return Ok(OpdsV1Response(feed.to_xml()));
    }

    if let Some(comic) = harmonia_db::repo::comic::get_comic(&state.db.read, &id_bytes).await? {
        let entry = comic_to_atom_entry(&comic);
        let feed = AtomFeed {
            id: entry.id.clone(),
            title: entry.title.clone(),
            updated: now,
            links: vec![AtomLink {
                rel: "self".to_string(),
                href: format!("/opds/v1/entry/{id}"),
                link_type: "application/atom+xml;profile=opds-catalog".to_string(),
                title: None,
            }],
            entries: vec![entry],
        };
        return Ok(OpdsV1Response(feed.to_xml()));
    }

    Err(ParocheError::NotFound)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;

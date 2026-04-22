use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use exousia::AuthenticatedUser;
use uuid::Uuid;

use crate::error::ParocheError;
use crate::state::AppState;

/// Serves the foliate-js reader SPA for a given book.
///
/// This handler renders an HTML page that:
/// - Loads the foliate-js library (ES modules, no bundler required)
/// - Fetches the book via the existing OPDS `/opds/content/:book_id` endpoint
/// - Streams the book into the `<foliate-view>` web component
///
/// The reader is served at `/read/:book_id` and requires authentication.
/// Book bytes are fetched from the existing OPDS endpoint which handles
/// Range requests (RFC 7233) transparently.
#[tracing::instrument(skip(state))]
pub async fn read_book(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ParocheError> {
    // Validate book ID format.
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    // Verify the book exists in the database.
    let book = apotheke::repo::book::get_book(&state.db.read, &id_bytes)
        .await
        .map_err(|e| ParocheError::Database { source: e })?
        .ok_or(ParocheError::NotFound)?;

    // Construct the reader HTML. The foliate-js bundle is vendored at
    // `crates/paroche/assets/reader/foliate-js-<SHA>/` and served via
    // `tower_http::services::ServeDir` mounted at `/static/reader/`.
    let foliate_js_sha = "76dcd8f0f7ccabd59199fc5eddbe012d8d463b18";
    let foliate_url = format!("/static/reader/foliate-js-{foliate_js_sha}");
    let book_content_url = format!("/opds/content/{id}");

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title} — Harmonia Reader</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        html, body {{
            width: 100%;
            height: 100%;
            font-family: system-ui, -apple-system, sans-serif;
        }}
        #reader {{
            width: 100%;
            height: 100%;
            display: flex;
            flex-direction: column;
        }}
        foliate-view {{
            flex: 1;
        }}
        .reader-toolbar {{
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 0.5rem 1rem;
            background: #f5f5f5;
            border-bottom: 1px solid #ddd;
            font-size: 0.9rem;
        }}
        .reader-toolbar h1 {{
            margin: 0;
            font-size: 1rem;
            flex: 1;
        }}
    </style>
</head>
<body>
    <div id="reader">
        <div class="reader-toolbar">
            <h1>{title}</h1>
            <span id="page-info">—</span>
        </div>
        <foliate-view id="book-view"></foliate-view>
    </div>

    <script type="module">
        // Import the foliate-js View component.
        import {{ View }} from "{foliate_url}/view.js";
        import EPUBBook from "{foliate_url}/epub.js";
        import MobiBook from "{foliate_url}/mobi.js";
        import FB2Book from "{foliate_url}/fb2.js";
        import ComicBook from "{foliate_url}/comic-book.js";

        const bookView = document.getElementById("book-view");
        const pageInfo = document.getElementById("page-info");

        // Fetch the book data from the OPDS endpoint.
        const contentUrl = "{book_content_url}";

        (async () => {{
            try {{
                const response = await fetch(contentUrl);
                if (!response.ok) {{
                    throw new Error(`Failed to fetch book: ${{response.status}}`);
                }}

                const blob = await response.blob();

                // Determine book type from Content-Type header.
                const contentType = response.headers.get("content-type") || "";
                const mimeType = contentType.split(";")[0];

                let BookClass;
                switch (mimeType) {{
                    case "application/epub+zip":
                        BookClass = EPUBBook;
                        break;
                    case "application/x-mobipocket-ebook":
                    case "application/vnd.amazon.ebook":
                        BookClass = MobiBook;
                        break;
                    case "application/x-fictionbook+xml":
                        BookClass = FB2Book;
                        break;
                    case "application/zip":
                    case "application/vnd.comicbook+zip":
                        BookClass = ComicBook;
                        break;
                    default:
                        // Try to infer from URL extension
                        const url = contentUrl.toLowerCase();
                        if (url.endsWith(".epub")) BookClass = EPUBBook;
                        else if (url.endsWith(".mobi") || url.endsWith(".azw3")) BookClass = MobiBook;
                        else if (url.endsWith(".fb2") || url.endsWith(".fb2.zip")) BookClass = FB2Book;
                        else if (url.endsWith(".cbz")) BookClass = ComicBook;
                        else {{
                            throw new Error("Could not determine book type from content-type or URL");
                        }}
                }}

                // Create a book instance from the blob.
                const book = new BookClass(blob);

                // Set the book in the view.
                bookView.book = book;

                // Update page info on navigation.
                bookView.addEventListener("relocated", (e) => {{
                    const progress = e.detail;
                    if (progress) {{
                        const percent = Math.round(progress * 100);
                        pageInfo.textContent = `${{percent}}%`;
                    }}
                }});

            }} catch (err) {{
                console.error(err);
                document.body.innerHTML = `<div style="padding: 2rem; color: red;">
                    Error loading book: ${{err.message}}
                </div>`;
            }}
        }})();
    </script>
</body>
</html>"#,
        title = html_escape(&book.title),
        foliate_url = foliate_url,
        book_content_url = book_content_url,
    );

    Ok((
        StatusCode::OK,
        [("Content-Type", "text/html; charset=utf-8")],
        html,
    ))
}

/// Simple HTML escaping to prevent injection.
fn html_escape(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '&' => "&amp;".chars().collect::<Vec<_>>(),
            '<' => "&lt;".chars().collect::<Vec<_>>(),
            '>' => "&gt;".chars().collect::<Vec<_>>(),
            '"' => "&quot;".chars().collect::<Vec<_>>(),
            '\'' => "&#39;".chars().collect::<Vec<_>>(),
            c => vec![c],
        })
        .collect()
}

pub fn reader_routes() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new().route("/read/{id}", get(read_book))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;
    use uuid::Uuid;

    use super::*;
    use crate::test_helpers::test_state;

    #[tokio::test]
    async fn read_nonexistent_book_returns_401_without_auth() {
        let (state, _auth) = test_state().await;
        let app = reader_routes().with_state(state);
        let id = Uuid::now_v7();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/read/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn html_escape_prevents_injection() {
        assert_eq!(html_escape("hello"), "hello");
        assert_eq!(html_escape("hello & world"), "hello &amp; world");
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    }
}

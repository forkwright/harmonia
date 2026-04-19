pub(crate) fn mime_from_format(format: Option<&str>) -> &'static str {
    match format {
        Some(f) => match f.to_lowercase().as_str() {
            "epub" => "application/epub+zip",
            "cbz" => "application/x-cbz",
            "cbr" => "application/x-cbr",
            "pdf" => "application/pdf",
            "mobi" | "azw3" => "application/x-mobipocket-ebook",
            _ => "application/octet-stream",
        },
        None => "application/octet-stream",
    }
}

pub(crate) fn mime_from_path(path: Option<&str>) -> &'static str {
    let ext = path
        .and_then(|p| std::path::Path::new(p).extension())
        .and_then(|e| e.to_str());
    mime_from_format(ext)
}

pub(crate) fn effective_mime(file_format: Option<&str>, file_path: Option<&str>) -> &'static str {
    if file_format.is_some() {
        mime_from_format(file_format)
    } else {
        mime_from_path(file_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epub_mime() {
        assert_eq!(mime_from_format(Some("epub")), "application/epub+zip");
    }

    #[test]
    fn cbz_mime() {
        assert_eq!(mime_from_format(Some("cbz")), "application/x-cbz");
    }

    #[test]
    fn pdf_mime() {
        assert_eq!(mime_from_format(Some("pdf")), "application/pdf");
    }

    #[test]
    fn unknown_format_returns_octet_stream() {
        assert_eq!(mime_from_format(Some("xyz")), "application/octet-stream");
    }

    #[test]
    fn none_format_returns_octet_stream() {
        assert_eq!(mime_from_format(None), "application/octet-stream");
    }

    #[test]
    fn mime_from_path_epub() {
        assert_eq!(
            mime_from_path(Some("/books/frank_herbert_dune.epub")),
            "application/epub+zip"
        );
    }

    #[test]
    fn effective_mime_prefers_format_over_path() {
        assert_eq!(
            effective_mime(Some("cbz"), Some("/comics/saga.epub")),
            "application/x-cbz"
        );
    }

    #[test]
    fn effective_mime_falls_back_to_path() {
        assert_eq!(
            effective_mime(None, Some("/books/dune.epub")),
            "application/epub+zip"
        );
    }
}

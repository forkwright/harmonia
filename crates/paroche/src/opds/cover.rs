//! Cover resolution: embedded archive covers (epub/cbz) first, sidecar
//! `cover.*` fallback, with byte-sniffed content types.

use std::io::Read;
use std::path::{Path as FsPath, PathBuf};

use quick_xml::events::Event;

// SAFETY: caps every cover/XML read — a hostile archive entry cannot balloon
// memory through a lying size field or an oversized document.
const MAX_COVER_BYTES: u64 = 20 * 1024 * 1024;
const MAX_XML_BYTES: u64 = 1024 * 1024;

/// Where a cover image was found for a media file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CoverLocation {
    /// An image entry inside the media file's own archive (epub/cbz).
    Embedded { archive: PathBuf, entry: String },
    /// A `cover.*` image beside the media file.
    Sidecar(PathBuf),
}

pub(crate) fn image_mime_from_ext(name: &str) -> Option<&'static str> {
    let ext = FsPath::new(name)
        .extension()?
        .to_str()?
        .to_ascii_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        _ => None,
    }
}

pub(crate) fn sniff_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        return Some("image/jpeg");
    }
    if bytes.get(0..4) == Some(b"RIFF".as_slice()) && bytes.get(8..12) == Some(b"WEBP".as_slice()) {
        return Some("image/webp");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    None
}

// NOTE: sidecar cover convention — a cover image lives beside the media file
// as `cover.{jpg,jpeg,png,webp}`; there is no dedicated cover column/table.
pub(crate) async fn find_sidecar_cover(media_file_path: &str) -> Option<PathBuf> {
    let parent = FsPath::new(media_file_path).parent()?;
    for ext in ["jpg", "jpeg", "png", "webp"] {
        let candidate = parent.join(format!("cover.{ext}"));
        if tokio::fs::try_exists(&candidate).await.unwrap_or(false) {
            return Some(candidate);
        }
    }
    None
}

#[derive(Debug, Clone, Copy)]
enum ArchiveKind {
    Epub,
    Cbz,
}

// NOTE: cbr (rar), pdf, and mobi/azw3 embedded covers are deferred — only
// the zip-container formats are probed; everything else falls to the sidecar.
fn archive_kind(file_format: Option<&str>, file_path: &str) -> Option<ArchiveKind> {
    let format = file_format.map(str::to_ascii_lowercase).or_else(|| {
        FsPath::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
    })?;
    match format.as_str() {
        "epub" => Some(ArchiveKind::Epub),
        "cbz" => Some(ArchiveKind::Cbz),
        _ => None,
    }
}

/// Locates the cover for a media file: an embedded archive cover when the
/// file is an epub/cbz, else a sidecar `cover.*` beside it.
pub(crate) async fn locate_cover(
    file_path: &str,
    file_format: Option<&str>,
) -> Option<CoverLocation> {
    if let Some(kind) = archive_kind(file_format, file_path) {
        let archive = PathBuf::from(file_path);
        match tokio::task::spawn_blocking(move || locate_embedded(&archive, kind)).await {
            Ok(Some(entry)) => {
                return Some(CoverLocation::Embedded {
                    archive: PathBuf::from(file_path),
                    entry,
                });
            }
            Ok(None) => {}
            Err(join_error) => {
                tracing::warn!(error = %join_error, path = %file_path, "embedded cover probe task failed");
            }
        }
    }
    find_sidecar_cover(file_path)
        .await
        .map(CoverLocation::Sidecar)
}

/// Reads the located cover's bytes and derives its content type by sniffing
/// the actual bytes, falling back to the file extension.
pub(crate) async fn read_cover(location: CoverLocation) -> Option<(Vec<u8>, &'static str)> {
    let (bytes, name) = match location {
        CoverLocation::Sidecar(path) => {
            let meta = tokio::fs::metadata(&path).await.ok()?;
            if meta.len() > MAX_COVER_BYTES {
                tracing::warn!(path = %path.display(), size = meta.len(), "sidecar cover exceeds size cap");
                return None;
            }
            let bytes = match tokio::fs::read(&path).await {
                Ok(bytes) => bytes,
                Err(error) => {
                    tracing::warn!(error = %error, path = %path.display(), "sidecar cover read failed");
                    return None;
                }
            };
            (bytes, path.to_string_lossy().into_owned())
        }
        CoverLocation::Embedded { archive, entry } => {
            let name = entry.clone();
            let bytes = tokio::task::spawn_blocking(move || read_embedded(&archive, &entry))
                .await
                .ok()
                .flatten()?;
            (bytes, name)
        }
    };
    let mime = sniff_image_mime(&bytes).or_else(|| image_mime_from_ext(&name));
    let Some(mime) = mime else {
        tracing::warn!(name = %name, "located cover has unrecognizable image type");
        return None;
    };
    Some((bytes, mime))
}

/// The content type to advertise for a media file's cover in catalog feeds,
/// or `None` when no cover is resolvable (the feed then omits image links).
///
/// PERF: extension-derived in the common case — one archive central-directory
/// probe per book, no image bytes read. A persisted cover reference produced
/// at import would remove the per-feed probe entirely (#580 follow-up).
pub(crate) async fn probe_cover_mime(
    file_path: &str,
    file_format: Option<&str>,
) -> Option<&'static str> {
    let location = locate_cover(file_path, file_format).await?;
    let ext_mime = match &location {
        CoverLocation::Embedded { entry, .. } => image_mime_from_ext(entry),
        CoverLocation::Sidecar(path) => path.to_str().and_then(image_mime_from_ext),
    };
    if let Some(mime) = ext_mime {
        return Some(mime);
    }
    // WHY: rare — e.g. an OPF-declared cover entry with no recognized
    // extension; read + sniff so the advertised type stays true to the bytes.
    read_cover(location).await.map(|(_, mime)| mime)
}

fn locate_embedded(path: &FsPath, kind: ArchiveKind) -> Option<String> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) => {
            tracing::warn!(error = %error, path = %path.display(), "cover probe could not open media file");
            return None;
        }
    };
    let mut archive = match zip::ZipArchive::new(file) {
        Ok(archive) => archive,
        Err(error) => {
            tracing::warn!(error = %error, path = %path.display(), "cover probe could not read archive");
            return None;
        }
    };
    match kind {
        ArchiveKind::Cbz => first_image_entry(&archive),
        ArchiveKind::Epub => epub_cover_entry(&mut archive),
    }
}

fn read_embedded(archive_path: &FsPath, entry: &str) -> Option<Vec<u8>> {
    let file = std::fs::File::open(archive_path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let entry_file = archive.by_name(entry).ok()?;
    let mut bytes = Vec::new();
    // WHY: take() bounds the read even when the declared entry size lies; an
    // over-cap read is discarded rather than served truncated.
    entry_file
        .take(MAX_COVER_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_COVER_BYTES {
        tracing::warn!(entry = %entry, "embedded cover exceeds size cap");
        return None;
    }
    Some(bytes)
}

fn is_hidden_entry(name: &str) -> bool {
    name.split('/')
        .any(|segment| segment.starts_with('.') || segment == "__MACOSX")
}

/// First image entry in page order — comic archives put the cover first.
fn first_image_entry<R: Read + std::io::Seek>(archive: &zip::ZipArchive<R>) -> Option<String> {
    archive
        .file_names()
        .filter(|name| !name.ends_with('/') && !is_hidden_entry(name))
        .filter(|name| image_mime_from_ext(name).is_some())
        .min()
        .map(str::to_string)
}

fn epub_cover_entry(archive: &mut zip::ZipArchive<std::fs::File>) -> Option<String> {
    if let Some(resolved) = opf_declared_cover(archive)
        && archive.by_name(&resolved).is_ok()
    {
        return Some(resolved);
    }
    // WHY: OPF hrefs occasionally disagree with the archive on case or
    // encoding; fall back to a conventional `cover.*` image entry.
    fallback_cover_entry(archive)
}

fn opf_declared_cover(archive: &mut zip::ZipArchive<std::fs::File>) -> Option<String> {
    let container = read_entry_string(archive, "META-INF/container.xml")?;
    let opf_path = parse_container_rootfile(&container)?;
    let opf = read_entry_string(archive, &opf_path)?;
    let href = parse_opf_cover_href(&opf)?;
    Some(resolve_opf_href(&opf_path, &href))
}

fn fallback_cover_entry<R: Read + std::io::Seek>(archive: &zip::ZipArchive<R>) -> Option<String> {
    archive
        .file_names()
        .filter(|name| !is_hidden_entry(name) && image_mime_from_ext(name).is_some())
        .filter(|name| {
            FsPath::new(name)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .is_some_and(|stem| stem.eq_ignore_ascii_case("cover"))
        })
        .min_by_key(|name| (name.matches('/').count(), name.to_string()))
        .map(str::to_string)
}

fn read_entry_string<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    name: &str,
) -> Option<String> {
    let entry = archive.by_name(name).ok()?;
    let mut text = String::new();
    entry
        .take(MAX_XML_BYTES + 1)
        .read_to_string(&mut text)
        .ok()?;
    if text.len() as u64 > MAX_XML_BYTES {
        return None;
    }
    Some(text)
}

fn attr_value(element: &quick_xml::events::BytesStart<'_>, name: &[u8]) -> Option<String> {
    element
        .attributes()
        .flatten()
        .find(|attr| attr.key.local_name().as_ref() == name)
        .and_then(|attr| {
            attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .ok()
        })
        .map(|value| value.into_owned())
}

/// `META-INF/container.xml` → the `full-path` of the first rootfile (the OPF).
fn parse_container_rootfile(xml: &str) -> Option<String> {
    let mut reader = quick_xml::Reader::from_str(xml);
    loop {
        match reader.read_event().ok()? {
            Event::Start(element) | Event::Empty(element) => {
                if element.name().local_name().as_ref() == b"rootfile"
                    && let Some(path) = attr_value(&element, b"full-path")
                {
                    return Some(path);
                }
            }
            Event::Eof => return None,
            _ => {}
        }
    }
}

/// The OPF-declared cover image href: EPUB 3 `properties="cover-image"`
/// manifest item, else the EPUB 2 `<meta name="cover">` item reference, else
/// the conventional `id="cover"` image item.
fn parse_opf_cover_href(xml: &str) -> Option<String> {
    struct ManifestItem {
        id: Option<String>,
        href: Option<String>,
        media_type: Option<String>,
        properties: Option<String>,
    }

    let mut items: Vec<ManifestItem> = Vec::new();
    let mut cover_meta_id: Option<String> = None;
    let mut reader = quick_xml::Reader::from_str(xml);
    loop {
        match reader.read_event().ok()? {
            Event::Start(element) | Event::Empty(element) => {
                match element.name().local_name().as_ref() {
                    b"item" => items.push(ManifestItem {
                        id: attr_value(&element, b"id"),
                        href: attr_value(&element, b"href"),
                        media_type: attr_value(&element, b"media-type"),
                        properties: attr_value(&element, b"properties"),
                    }),
                    b"meta" => {
                        if attr_value(&element, b"name").as_deref() == Some("cover")
                            && let Some(content) = attr_value(&element, b"content")
                        {
                            cover_meta_id = Some(content);
                        }
                    }
                    _ => {}
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    if let Some(item) = items.iter().find(|item| {
        item.properties
            .as_deref()
            .is_some_and(|p| p.split_whitespace().any(|token| token == "cover-image"))
    }) {
        return item.href.clone();
    }
    if let Some(cover_id) = cover_meta_id
        && let Some(item) = items
            .iter()
            .find(|item| item.id.as_deref() == Some(&cover_id))
    {
        return item.href.clone();
    }
    items
        .iter()
        .find(|item| {
            item.id.as_deref() == Some("cover")
                && item
                    .media_type
                    .as_deref()
                    .is_some_and(|m| m.starts_with("image/"))
        })
        .and_then(|item| item.href.clone())
}

/// Decodes `%XX` sequences; malformed sequences pass through verbatim.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while let Some(&byte) = bytes.get(i) {
        if byte == b'%'
            && let (Some(hi), Some(lo)) = (
                bytes.get(i + 1).and_then(|b| (*b as char).to_digit(16)),
                bytes.get(i + 2).and_then(|b| (*b as char).to_digit(16)),
            )
        {
            // INVARIANT: hi/lo are hex digits (< 16), so the sum fits a u8.
            out.push((hi * 16 + lo) as u8);
            i += 3;
        } else {
            out.push(byte);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| input.to_string())
}

/// Resolves an OPF-relative href against the OPF's own archive path.
fn resolve_opf_href(opf_path: &str, href: &str) -> String {
    let decoded = percent_decode(href);
    let mut segments: Vec<&str> = opf_path.split('/').collect();
    segments.pop();
    for segment in decoded.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }
    segments.join("/")
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::TempDir;

    use super::*;

    const PNG_BYTES: &[u8] = b"\x89PNG\r\n\x1a\n-fake-png-body";
    const JPEG_BYTES: &[u8] = b"\xff\xd8\xff\xe0-fake-jpeg-body";

    fn zip_options() -> zip::write::SimpleFileOptions {
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored)
    }

    fn write_zip(path: &FsPath, entries: &[(&str, &[u8])]) {
        let file = std::fs::File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        for (name, bytes) in entries {
            writer.start_file(*name, zip_options()).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
    }

    fn epub_entries<'a>(opf: &'a str, image: (&'a str, &'a [u8])) -> Vec<(&'a str, &'a [u8])> {
        vec![
            (
                "META-INF/container.xml",
                br#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"# as &[u8],
            ),
            ("OEBPS/content.opf", opf.as_bytes()),
            image,
        ]
    }

    #[test]
    fn sniff_recognizes_magic_bytes() {
        assert_eq!(sniff_image_mime(PNG_BYTES), Some("image/png"));
        assert_eq!(sniff_image_mime(JPEG_BYTES), Some("image/jpeg"));
        assert_eq!(
            sniff_image_mime(b"RIFF\x00\x00\x00\x00WEBPVP8 "),
            Some("image/webp")
        );
        assert_eq!(sniff_image_mime(b"GIF89a-body"), Some("image/gif"));
        assert_eq!(sniff_image_mime(b"not-an-image"), None);
    }

    #[test]
    fn ext_mime_maps_known_extensions() {
        assert_eq!(image_mime_from_ext("cover.PNG"), Some("image/png"));
        assert_eq!(image_mime_from_ext("a/b/cover.jpeg"), Some("image/jpeg"));
        assert_eq!(image_mime_from_ext("cover.webp"), Some("image/webp"));
        assert_eq!(image_mime_from_ext("cover.txt"), None);
        assert_eq!(image_mime_from_ext("no-extension"), None);
    }

    #[test]
    fn percent_decode_handles_encoded_and_malformed() {
        assert_eq!(percent_decode("cover%20image.jpg"), "cover image.jpg");
        assert_eq!(percent_decode("plain.jpg"), "plain.jpg");
        assert_eq!(percent_decode("bad%zz.jpg"), "bad%zz.jpg");
    }

    #[test]
    fn resolve_opf_href_joins_and_normalizes() {
        assert_eq!(
            resolve_opf_href("OEBPS/content.opf", "images/cover.jpg"),
            "OEBPS/images/cover.jpg"
        );
        assert_eq!(
            resolve_opf_href("OEBPS/content.opf", "../cover.jpg"),
            "cover.jpg"
        );
        assert_eq!(resolve_opf_href("content.opf", "cover.jpg"), "cover.jpg");
        assert_eq!(
            resolve_opf_href("OEBPS/content.opf", "./images/c%20d.png"),
            "OEBPS/images/c d.png"
        );
    }

    #[test]
    fn container_rootfile_parses() {
        let xml = r#"<container><rootfiles>
            <rootfile full-path="OEBPS/package.opf" media-type="application/oebps-package+xml"/>
        </rootfiles></container>"#;
        assert_eq!(
            parse_container_rootfile(xml),
            Some("OEBPS/package.opf".to_string())
        );
        assert_eq!(parse_container_rootfile("<container/>"), None);
    }

    #[test]
    fn opf_cover_prefers_epub3_properties() {
        let xml = r#"<package><manifest>
            <item id="c2" href="old.jpg" media-type="image/jpeg"/>
            <item id="c3" href="new.png" media-type="image/png" properties="svg cover-image"/>
        </manifest><metadata><meta name="cover" content="c2"/></metadata></package>"#;
        assert_eq!(parse_opf_cover_href(xml), Some("new.png".to_string()));
    }

    #[test]
    fn opf_cover_falls_back_to_epub2_meta() {
        let xml = r#"<package><metadata><meta name="cover" content="cover-id"/></metadata>
        <manifest><item id="cover-id" href="images/cover.jpg" media-type="image/jpeg"/></manifest>
        </package>"#;
        assert_eq!(
            parse_opf_cover_href(xml),
            Some("images/cover.jpg".to_string())
        );
    }

    #[test]
    fn opf_cover_falls_back_to_cover_id_convention() {
        let xml = r#"<package><manifest>
            <item id="cover" href="cover.webp" media-type="image/webp"/>
            <item id="page1" href="p1.xhtml" media-type="application/xhtml+xml"/>
        </manifest></package>"#;
        assert_eq!(parse_opf_cover_href(xml), Some("cover.webp".to_string()));
    }

    #[test]
    fn opf_without_cover_returns_none() {
        let xml = r#"<package><manifest>
            <item id="page1" href="p1.xhtml" media-type="application/xhtml+xml"/>
        </manifest></package>"#;
        assert_eq!(parse_opf_cover_href(xml), None);
    }

    #[tokio::test]
    async fn cbz_cover_is_first_image_entry() {
        let dir = TempDir::new().unwrap();
        let cbz = dir.path().join("comic.cbz");
        write_zip(
            &cbz,
            &[
                ("ComicInfo.xml", b"<ComicInfo/>" as &[u8]),
                ("pages/002.png", PNG_BYTES),
                ("pages/001.png", PNG_BYTES),
                ("__MACOSX/._001.png", b"junk"),
            ],
        );
        let location = locate_cover(cbz.to_str().unwrap(), Some("cbz"))
            .await
            .unwrap();
        assert_eq!(
            location,
            CoverLocation::Embedded {
                archive: cbz.clone(),
                entry: "pages/001.png".to_string()
            }
        );
        let (bytes, mime) = read_cover(location).await.unwrap();
        assert_eq!(mime, "image/png");
        assert_eq!(bytes, PNG_BYTES);
    }

    #[tokio::test]
    async fn epub_cover_resolves_via_opf() {
        let dir = TempDir::new().unwrap();
        let epub = dir.path().join("book.epub");
        let opf = r#"<package><manifest>
            <item id="ci" href="images/cover.jpeg" media-type="image/jpeg" properties="cover-image"/>
        </manifest></package>"#;
        write_zip(
            &epub,
            &epub_entries(opf, ("OEBPS/images/cover.jpeg", JPEG_BYTES)),
        );
        let location = locate_cover(epub.to_str().unwrap(), Some("epub"))
            .await
            .unwrap();
        assert_eq!(
            location,
            CoverLocation::Embedded {
                archive: epub.clone(),
                entry: "OEBPS/images/cover.jpeg".to_string()
            }
        );
        let (_, mime) = read_cover(location).await.unwrap();
        assert_eq!(mime, "image/jpeg");
    }

    #[tokio::test]
    async fn epub_without_opf_cover_uses_conventional_entry() {
        let dir = TempDir::new().unwrap();
        let epub = dir.path().join("book.epub");
        let opf = r#"<package><manifest>
            <item id="page1" href="p1.xhtml" media-type="application/xhtml+xml"/>
        </manifest></package>"#;
        write_zip(&epub, &epub_entries(opf, ("OEBPS/cover.png", PNG_BYTES)));
        let location = locate_cover(epub.to_str().unwrap(), Some("epub"))
            .await
            .unwrap();
        assert_eq!(
            location,
            CoverLocation::Embedded {
                archive: epub.clone(),
                entry: "OEBPS/cover.png".to_string()
            }
        );
    }

    #[tokio::test]
    async fn non_archive_falls_back_to_sidecar() {
        let dir = TempDir::new().unwrap();
        let media = dir.path().join("book.pdf");
        tokio::fs::write(&media, b"%PDF-1.4").await.unwrap();
        let sidecar = dir.path().join("cover.png");
        tokio::fs::write(&sidecar, PNG_BYTES).await.unwrap();
        let location = locate_cover(media.to_str().unwrap(), Some("pdf"))
            .await
            .unwrap();
        assert_eq!(location, CoverLocation::Sidecar(sidecar));
    }

    #[tokio::test]
    async fn corrupt_archive_falls_back_to_sidecar() {
        let dir = TempDir::new().unwrap();
        let media = dir.path().join("book.epub");
        tokio::fs::write(&media, b"not-a-zip-archive")
            .await
            .unwrap();
        let sidecar = dir.path().join("cover.jpg");
        tokio::fs::write(&sidecar, JPEG_BYTES).await.unwrap();
        let location = locate_cover(media.to_str().unwrap(), Some("epub"))
            .await
            .unwrap();
        assert_eq!(location, CoverLocation::Sidecar(sidecar));
    }

    #[tokio::test]
    async fn sidecar_sniff_wins_over_extension() {
        // WHY: a mislabeled sidecar (PNG bytes in cover.jpg) must be served
        // as what it IS, not what its name claims.
        let dir = TempDir::new().unwrap();
        let sidecar = dir.path().join("cover.jpg");
        tokio::fs::write(&sidecar, PNG_BYTES).await.unwrap();
        let (_, mime) = read_cover(CoverLocation::Sidecar(sidecar)).await.unwrap();
        assert_eq!(mime, "image/png");
    }

    #[tokio::test]
    async fn missing_cover_resolves_to_none() {
        let dir = TempDir::new().unwrap();
        let media = dir.path().join("book.pdf");
        tokio::fs::write(&media, b"%PDF-1.4").await.unwrap();
        assert!(
            locate_cover(media.to_str().unwrap(), Some("pdf"))
                .await
                .is_none()
        );
        assert!(
            probe_cover_mime(media.to_str().unwrap(), Some("pdf"))
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn probe_mime_matches_located_cover() {
        let dir = TempDir::new().unwrap();
        let cbz = dir.path().join("comic.cbz");
        write_zip(
            &cbz,
            &[("001.webp", b"RIFF\x00\x00\x00\x00WEBPVP8 " as &[u8])],
        );
        assert_eq!(
            probe_cover_mime(cbz.to_str().unwrap(), Some("cbz")).await,
            Some("image/webp")
        );
    }
}

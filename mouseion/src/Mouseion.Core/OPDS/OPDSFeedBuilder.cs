// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Xml.Linq;
using Mouseion.Core.MediaItems;

namespace Mouseion.Core.OPDS;

/// <summary>
/// Builds OPDS 1.2 Atom feeds for books, comics, and manga.
/// Spec: https://specs.opds.io/opds-1.2
/// </summary>
public interface IOPDSFeedBuilder
{
    XDocument BuildCatalogRoot(string baseUrl);
    XDocument BuildNavigationFeed(string baseUrl, string mediaType, string title);
    XDocument BuildAcquisitionFeed(string baseUrl, IEnumerable<MediaItemSummary> items, string title, int page, int pageSize, int totalItems);
    XDocument BuildSearchDescriptor(string baseUrl);
    XElement BuildEntry(string baseUrl, MediaItemSummary item);
}

public class OPDSFeedBuilder : IOPDSFeedBuilder
{
    private static readonly XNamespace Atom = "http://www.w3.org/2005/Atom";
    private static readonly XNamespace Opds = "http://opds-spec.org/2010/catalog";
    private static readonly XNamespace DcTerms = "http://purl.org/dc/terms/";
    private static readonly XNamespace OpenSearch = "http://a9.com/-/spec/opensearch/1.1/";
    private static readonly XNamespace Pse = "http://vaemendis.net/opds-pse/ns";

    public XDocument BuildCatalogRoot(string baseUrl)
    {
        var feed = CreateFeed(
            id: $"{baseUrl}/opds/v1.2/catalog",
            title: "Mouseion Library",
            updated: DateTime.UtcNow);

        feed.Root!.Add(
            CreateNavigationEntry(baseUrl, "books", "Books", "All books in your library"),
            CreateNavigationEntry(baseUrl, "comics", "Comics", "All comics in your library"),
            CreateNavigationEntry(baseUrl, "manga", "Manga", "All manga in your library"),
            CreateNavigationEntry(baseUrl, "recent", "Recently Added", "Recently added items across all types"),
            new XElement(Atom + "link",
                new XAttribute("rel", "self"),
                new XAttribute("type", "application/atom+xml;profile=opds-catalog;kind=navigation"),
                new XAttribute("href", $"{baseUrl}/opds/v1.2/catalog")),
            new XElement(Atom + "link",
                new XAttribute("rel", "start"),
                new XAttribute("type", "application/atom+xml;profile=opds-catalog;kind=navigation"),
                new XAttribute("href", $"{baseUrl}/opds/v1.2/catalog")),
            new XElement(Atom + "link",
                new XAttribute("rel", "search"),
                new XAttribute("type", "application/opensearchdescription+xml"),
                new XAttribute("href", $"{baseUrl}/opds/v1.2/search.xml")));

        return feed;
    }

    public XDocument BuildNavigationFeed(string baseUrl, string mediaType, string title)
    {
        var feed = CreateFeed(
            id: $"{baseUrl}/opds/v1.2/{mediaType}",
            title: title,
            updated: DateTime.UtcNow);

        feed.Root!.Add(
            new XElement(Atom + "link",
                new XAttribute("rel", "self"),
                new XAttribute("type", "application/atom+xml;profile=opds-catalog;kind=navigation"),
                new XAttribute("href", $"{baseUrl}/opds/v1.2/{mediaType}")),
            new XElement(Atom + "link",
                new XAttribute("rel", "start"),
                new XAttribute("type", "application/atom+xml;profile=opds-catalog;kind=navigation"),
                new XAttribute("href", $"{baseUrl}/opds/v1.2/catalog")),
            CreateNavigationEntry(baseUrl, $"{mediaType}/all", "All", $"Browse all {title.ToLowerInvariant()}"),
            CreateNavigationEntry(baseUrl, $"{mediaType}/recent", "Recently Added", $"Recently added {title.ToLowerInvariant()}"));

        return feed;
    }

    public XDocument BuildAcquisitionFeed(string baseUrl, IEnumerable<MediaItemSummary> items, string title, int page, int pageSize, int totalItems)
    {
        var feedPath = title.ToLowerInvariant().Replace(' ', '-');
        var feed = CreateFeed(
            id: $"{baseUrl}/opds/v1.2/{feedPath}",
            title: title,
            updated: DateTime.UtcNow);

        // Self + start links
        feed.Root!.Add(
            new XElement(Atom + "link",
                new XAttribute("rel", "self"),
                new XAttribute("type", "application/atom+xml;profile=opds-catalog;kind=acquisition"),
                new XAttribute("href", $"{baseUrl}/opds/v1.2/{feedPath}?page={page}&pageSize={pageSize}")),
            new XElement(Atom + "link",
                new XAttribute("rel", "start"),
                new XAttribute("type", "application/atom+xml;profile=opds-catalog;kind=navigation"),
                new XAttribute("href", $"{baseUrl}/opds/v1.2/catalog")));

        // Pagination (OpenSearch)
        feed.Root.Add(
            new XElement(OpenSearch + "totalResults", totalItems),
            new XElement(OpenSearch + "startIndex", (page - 1) * pageSize),
            new XElement(OpenSearch + "itemsPerPage", pageSize));

        if (page > 1)
        {
            feed.Root.Add(new XElement(Atom + "link",
                new XAttribute("rel", "previous"),
                new XAttribute("type", "application/atom+xml;profile=opds-catalog;kind=acquisition"),
                new XAttribute("href", $"{baseUrl}/opds/v1.2/{feedPath}?page={page - 1}&pageSize={pageSize}")));
        }

        var totalPages = (int)Math.Ceiling((double)totalItems / pageSize);
        if (page < totalPages)
        {
            feed.Root.Add(new XElement(Atom + "link",
                new XAttribute("rel", "next"),
                new XAttribute("type", "application/atom+xml;profile=opds-catalog;kind=acquisition"),
                new XAttribute("href", $"{baseUrl}/opds/v1.2/{feedPath}?page={page + 1}&pageSize={pageSize}")));
        }

        foreach (var item in items)
        {
            feed.Root.Add(BuildEntry(baseUrl, item));
        }

        return feed;
    }

    public XDocument BuildSearchDescriptor(string baseUrl)
    {
        var ns = XNamespace.Get("http://a9.com/-/spec/opensearch/1.1/");
        return new XDocument(
            new XDeclaration("1.0", "utf-8", null),
            new XElement(ns + "OpenSearchDescription",
                new XElement(ns + "ShortName", "Mouseion"),
                new XElement(ns + "Description", "Search the Mouseion library"),
                new XElement(ns + "InputEncoding", "UTF-8"),
                new XElement(ns + "OutputEncoding", "UTF-8"),
                new XElement(ns + "Url",
                    new XAttribute("type", "application/atom+xml;profile=opds-catalog;kind=acquisition"),
                    new XAttribute("template", $"{baseUrl}/opds/v1.2/search?q={{searchTerms}}&page={{startPage?}}"))));
    }

    public XElement BuildEntry(string baseUrl, MediaItemSummary item)
    {
        var entry = new XElement(Atom + "entry",
            new XElement(Atom + "id", $"urn:mouseion:media:{item.Id}"),
            new XElement(Atom + "title", item.Title),
            new XElement(Atom + "updated", (item.LastModified ?? item.Added).ToString("o")),
            new XElement(Atom + "content",
                new XAttribute("type", "text"),
                $"{item.MediaType} — Added {item.Added:yyyy-MM-dd}"));

        // Category (media type)
        entry.Add(new XElement(Atom + "category",
            new XAttribute("scheme", "http://www.bisg.org/standards/bisac_subject/"),
            new XAttribute("term", item.MediaType.ToString()),
            new XAttribute("label", item.MediaType.ToString())));

        // Acquisition link (direct file download)
        var mimeType = GetMimeType(item.Path);
        if (!string.IsNullOrEmpty(item.Path))
        {
            entry.Add(new XElement(Atom + "link",
                new XAttribute("rel", "http://opds-spec.org/acquisition"),
                new XAttribute("type", mimeType),
                new XAttribute("href", $"{baseUrl}/api/v3/mediafiles/{item.Id}/download")));
        }

        // Cover image links
        entry.Add(
            new XElement(Atom + "link",
                new XAttribute("rel", "http://opds-spec.org/image"),
                new XAttribute("type", "image/jpeg"),
                new XAttribute("href", $"{baseUrl}/api/v3/mediafiles/{item.Id}/cover")),
            new XElement(Atom + "link",
                new XAttribute("rel", "http://opds-spec.org/image/thumbnail"),
                new XAttribute("type", "image/jpeg"),
                new XAttribute("href", $"{baseUrl}/api/v3/mediafiles/{item.Id}/cover?size=thumbnail")));

        return entry;
    }

    private static XDocument CreateFeed(string id, string title, DateTime updated)
    {
        return new XDocument(
            new XDeclaration("1.0", "utf-8", null),
            new XElement(Atom + "feed",
                new XAttribute(XNamespace.Xmlns + "dc", DcTerms),
                new XAttribute(XNamespace.Xmlns + "opds", Opds),
                new XAttribute(XNamespace.Xmlns + "opensearch", OpenSearch),
                new XAttribute(XNamespace.Xmlns + "pse", Pse),
                new XElement(Atom + "id", id),
                new XElement(Atom + "title", title),
                new XElement(Atom + "updated", updated.ToString("o")),
                new XElement(Atom + "author",
                    new XElement(Atom + "name", "Mouseion"),
                    new XElement(Atom + "uri", "https://github.com/forkwright/mouseion"))));
    }

    private static XElement CreateNavigationEntry(string baseUrl, string path, string title, string summary)
    {
        return new XElement(Atom + "entry",
            new XElement(Atom + "id", $"{baseUrl}/opds/v1.2/{path}"),
            new XElement(Atom + "title", title),
            new XElement(Atom + "content",
                new XAttribute("type", "text"),
                summary),
            new XElement(Atom + "updated", DateTime.UtcNow.ToString("o")),
            new XElement(Atom + "link",
                new XAttribute("rel", "subsection"),
                new XAttribute("type", "application/atom+xml;profile=opds-catalog;kind=navigation"),
                new XAttribute("href", $"{baseUrl}/opds/v1.2/{path}")));
    }

    private static string GetMimeType(string? path)
    {
        var ext = System.IO.Path.GetExtension(path)?.ToLowerInvariant();
        return ext switch
        {
            ".epub" => "application/epub+zip",
            ".pdf" => "application/pdf",
            ".cbz" => "application/vnd.comicbook+zip",
            ".cbr" => "application/vnd.comicbook-rar",
            ".cb7" => "application/x-cb7",
            ".mobi" => "application/x-mobipocket-ebook",
            ".azw" or ".azw3" => "application/vnd.amazon.ebook",
            ".fb2" => "application/x-fictionbook+xml",
            _ => "application/octet-stream"
        };
    }
}

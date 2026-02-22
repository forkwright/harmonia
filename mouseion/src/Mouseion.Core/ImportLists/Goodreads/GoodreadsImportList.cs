// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Globalization;
using System.Net.Http;
using System.Xml.Linq;
using Microsoft.Extensions.Logging;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.ImportLists.Goodreads;

/// <summary>
/// Imports books from Goodreads via RSS shelf feeds.
/// Goodreads deprecated their API in December 2020, but RSS feeds for shelves
/// remain functional and are the standard import method used by competing tools.
/// Feed URL pattern: https://www.goodreads.com/review/list_rss/{userId}?shelf={shelf}
/// Each feed returns up to 200 items per page.
/// </summary>
public class GoodreadsImportList : ImportListBase<GoodreadsSettings>
{
    private readonly IHttpClientFactory _httpClientFactory;

    // Goodreads formats that indicate audiobook editions
    private static readonly HashSet<string> AudiobookFormats = new(StringComparer.OrdinalIgnoreCase)
    {
        "Audio CD", "Audiobook", "Audio Cassette", "Audible Audio",
        "MP3 CD", "Audio", "Unabridged Audio"
    };

    public GoodreadsImportList(ILogger<GoodreadsImportList> logger, IHttpClientFactory httpClientFactory)
        : base(logger)
    {
        _httpClientFactory = httpClientFactory;
    }

    public override string Name => "Goodreads";
    public override ImportListType ListType => ImportListType.Goodreads;
    public override TimeSpan MinRefreshInterval => TimeSpan.FromHours(6);
    public override bool Enabled => Settings.IsConfigured;
    public override bool EnableAuto => Settings.IsConfigured;

    public override async Task<ImportListFetchResult> FetchAsync(CancellationToken cancellationToken = default)
    {
        var result = new ImportListFetchResult();
        var allItems = new List<ImportListItem>();

        try
        {
            if (!Settings.IsConfigured)
            {
                Logger.LogWarning("Goodreads user ID not configured");
                result.AnyFailure = true;
                return result;
            }

            var client = _httpClientFactory.CreateClient();
            client.DefaultRequestHeaders.Add("User-Agent", "Mouseion/1.0");
            var shelfCount = 0;

            foreach (var shelf in Settings.Shelves)
            {
                var mapping = Settings.ShelfMappings.GetValueOrDefault(shelf, GoodreadsShelfMapping.Monitored);

                if (mapping == GoodreadsShelfMapping.Ignored)
                {
                    Logger.LogDebug("Skipping ignored shelf: {Shelf}", shelf);
                    continue;
                }

                try
                {
                    var shelfItems = await FetchShelfAsync(client, shelf, mapping, cancellationToken)
                        .ConfigureAwait(false);
                    allItems.AddRange(shelfItems);
                    shelfCount++;
                }
                catch (HttpRequestException ex)
                {
                    Logger.LogWarning(ex, "Failed to fetch Goodreads shelf: {Shelf}", shelf);
                    // Continue with other shelves — partial success is better than total failure
                }
            }

            // Deduplicate by Goodreads book ID (same book can appear in multiple shelves)
            result.Items = DeduplicateByGoodreadsId(allItems);
            result.SyncedLists = shelfCount;

            Logger.LogInformation(
                "Goodreads import fetched {Count} unique items from {ShelfCount} shelves",
                result.Items.Count, shelfCount);
        }
        catch (Exception ex) when (ex is not OperationCanceledException)
        {
            Logger.LogError(ex, "Failed to complete Goodreads import");
            result.AnyFailure = true;
        }

        return result;
    }

    private async Task<List<ImportListItem>> FetchShelfAsync(
        HttpClient client, string shelf, GoodreadsShelfMapping mapping,
        CancellationToken cancellationToken)
    {
        var items = new List<ImportListItem>();
        var page = 1;
        bool hasMore;

        do
        {
            var url = $"{Settings.BaseUrl}/review/list_rss/{Settings.UserId}?shelf={shelf}&page={page}";
            var response = await client.GetAsync(url, cancellationToken).ConfigureAwait(false);

            if (response.StatusCode == System.Net.HttpStatusCode.NotFound)
            {
                Logger.LogWarning("Goodreads shelf not found or profile is private: {Shelf}", shelf);
                return items;
            }

            response.EnsureSuccessStatusCode();
            var xml = await response.Content.ReadAsStringAsync(cancellationToken).ConfigureAwait(false);

            var parsed = ParseRssFeed(xml);
            hasMore = parsed.Count >= 200; // Goodreads returns max 200 per page

            foreach (var rssItem in parsed)
            {
                var item = MapToImportListItem(rssItem, shelf, mapping);
                if (item != null)
                {
                    items.Add(item);
                }
            }

            page++;

            // Safety cap — don't paginate forever
            if (page > 50)
            {
                Logger.LogWarning("Hit pagination limit (10,000 items) for shelf {Shelf}", shelf);
                break;
            }
        }
        while (hasMore);

        return items;
    }

    internal List<GoodreadsRssItem> ParseRssFeed(string xml)
    {
        var items = new List<GoodreadsRssItem>();

        try
        {
            var doc = XDocument.Parse(xml);
            var channel = doc.Root?.Element("channel");

            if (channel == null)
            {
                Logger.LogWarning("Goodreads RSS feed has no channel element");
                return items;
            }

            foreach (var itemElement in channel.Elements("item"))
            {
                var rssItem = new GoodreadsRssItem
                {
                    Title = CleanCData(itemElement.Element("title")?.Value),
                    AuthorName = CleanCData(itemElement.Element("author_name")?.Value),
                    BookId = ParseLong(itemElement.Element("book_id")?.Value),
                    Isbn = NullIfEmpty(CleanCData(itemElement.Element("isbn")?.Value)),
                    Isbn13 = NullIfEmpty(CleanCData(itemElement.Element("isbn13")?.Value)),
                    UserRating = ParseNullableInt(itemElement.Element("user_rating")?.Value),
                    AverageRating = ParseNullableInt(itemElement.Element("average_rating")?.Value),
                    BookPublished = NullIfEmpty(CleanCData(itemElement.Element("book_published")?.Value)),
                    ImageUrl = NullIfEmpty(CleanCData(itemElement.Element("book_large_image_url")?.Value)),
                    Format = NullIfEmpty(CleanCData(itemElement.Element("book_format")?.Value)),
                    NumPages = ParseNullableInt(itemElement.Element("num_pages")?.Value),
                    UserReadAt = ParseGoodreadsDate(itemElement.Element("user_read_at")?.Value),
                    UserDateAdded = ParseGoodreadsDate(itemElement.Element("user_date_added")?.Value),
                    UserReview = NullIfEmpty(CleanCData(itemElement.Element("user_review")?.Value))
                };

                if (rssItem.BookId > 0)
                {
                    items.Add(rssItem);
                }
            }
        }
        catch (Exception ex)
        {
            Logger.LogError(ex, "Failed to parse Goodreads RSS XML");
        }

        return items;
    }

    private ImportListItem? MapToImportListItem(
        GoodreadsRssItem rssItem, string shelf, GoodreadsShelfMapping mapping)
    {
        if (string.IsNullOrEmpty(rssItem.Title))
            return null;

        var isAudiobook = Settings.DetectAudiobooks
            && !string.IsNullOrEmpty(rssItem.Format)
            && AudiobookFormats.Contains(rssItem.Format);

        var item = new ImportListItem
        {
            ListId = Definition.Id,
            MediaType = isAudiobook ? MediaType.Audiobook : MediaType.Book,
            Title = rssItem.Title,
            Author = rssItem.AuthorName,
            GoodreadsId = rssItem.BookId,
            Isbn = rssItem.Isbn13 ?? rssItem.Isbn, // Prefer ISBN-13
            Year = ParseYear(rssItem.BookPublished),
            ImportSource = $"goodreads:{shelf}",
            WatchedAt = rssItem.UserReadAt
        };

        // Map rating: Goodreads uses 1-5 stars, scale to 1-10
        if (Settings.ImportRatings && rssItem.UserRating.HasValue && rssItem.UserRating.Value > 0)
        {
            item.UserRating = rssItem.UserRating.Value * 2;
        }

        return item;
    }

    private static List<ImportListItem> DeduplicateByGoodreadsId(List<ImportListItem> items)
    {
        var seen = new HashSet<long>();
        var result = new List<ImportListItem>();

        foreach (var item in items)
        {
            if (item.GoodreadsId == 0 || seen.Add(item.GoodreadsId))
            {
                result.Add(item);
            }
        }

        return result;
    }

    #region Parsing Helpers

    private static string CleanCData(string? value)
    {
        if (string.IsNullOrEmpty(value))
            return string.Empty;

        // Goodreads wraps most fields in CDATA — XDocument handles that,
        // but we still need to trim whitespace
        return value.Trim();
    }

    private static string? NullIfEmpty(string? value)
    {
        return string.IsNullOrWhiteSpace(value) ? null : value;
    }

    private static long ParseLong(string? value)
    {
        return long.TryParse(value?.Trim(), out var result) ? result : 0;
    }

    private static int? ParseNullableInt(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
            return null;

        return int.TryParse(value.Trim(), out var result) ? result : null;
    }

    private static int ParseYear(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
            return 0;

        // BookPublished can be just a year ("2019") or a full date
        if (int.TryParse(value.Trim(), out var year) && year > 0 && year < 3000)
            return year;

        if (DateTime.TryParse(value.Trim(), CultureInfo.InvariantCulture, DateTimeStyles.None, out var date))
            return date.Year;

        return 0;
    }

    private static DateTime? ParseGoodreadsDate(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
            return null;

        // Goodreads uses RFC 2822 dates: "Sat, 01 Jan 2025 12:00:00 -0800"
        if (DateTime.TryParse(value.Trim(), CultureInfo.InvariantCulture, DateTimeStyles.AdjustToUniversal, out var date))
            return date;

        return null;
    }

    #endregion
}

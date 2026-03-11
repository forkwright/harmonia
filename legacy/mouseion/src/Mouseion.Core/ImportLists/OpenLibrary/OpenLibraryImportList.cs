// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Globalization;
using System.Net.Http;
using System.Net.Http.Headers;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.ImportLists.OpenLibrary;

/// <summary>
/// Imports books from OpenLibrary reading logs via their public JSON API.
/// No authentication required — reading logs are public.
/// API: https://openlibrary.org/people/{username}/books/{shelf}.json
/// Rate limit: ~100 requests/5 minutes (be respectful — it's a nonprofit).
/// </summary>
public class OpenLibraryImportList : ImportListBase<OpenLibrarySettings>
{
    private readonly IHttpClientFactory _httpClientFactory;
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true
    };

    public OpenLibraryImportList(ILogger<OpenLibraryImportList> logger, IHttpClientFactory httpClientFactory)
        : base(logger)
    {
        _httpClientFactory = httpClientFactory;
    }

    public override string Name => "OpenLibrary";
    public override ImportListType ListType => ImportListType.OpenLibrary;
    public override TimeSpan MinRefreshInterval => TimeSpan.FromHours(12);
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
                Logger.LogWarning("OpenLibrary username not configured");
                result.AnyFailure = true;
                return result;
            }

            var client = CreateClient();
            var shelfCount = 0;

            if (Settings.ImportAlreadyRead)
            {
                var items = await FetchShelfAsync(client, "already-read", cancellationToken).ConfigureAwait(false);
                foreach (var item in items)
                {
                    item.ImportSource = "openlibrary:already-read";
                }
                allItems.AddRange(items);
                shelfCount++;
            }

            if (Settings.ImportCurrentlyReading)
            {
                var items = await FetchShelfAsync(client, "currently-reading", cancellationToken).ConfigureAwait(false);
                foreach (var item in items)
                {
                    item.ImportSource = "openlibrary:currently-reading";
                }
                allItems.AddRange(items);
                shelfCount++;
            }

            if (Settings.ImportWantToRead)
            {
                var items = await FetchShelfAsync(client, "want-to-read", cancellationToken).ConfigureAwait(false);
                foreach (var item in items)
                {
                    item.ImportSource = "openlibrary:want-to-read";
                }
                allItems.AddRange(items);
                shelfCount++;
            }

            // Deduplicate by OL work key
            result.Items = DeduplicateByWorkKey(allItems);
            result.SyncedLists = shelfCount;

            Logger.LogInformation(
                "OpenLibrary import fetched {Count} unique items from {ShelfCount} shelves for user {Username}",
                result.Items.Count, shelfCount, Settings.Username);
        }
        catch (Exception ex) when (ex is not OperationCanceledException)
        {
            Logger.LogError(ex, "Failed to complete OpenLibrary import");
            result.AnyFailure = true;
        }

        return result;
    }

    private async Task<List<ImportListItem>> FetchShelfAsync(
        HttpClient client, string shelf, CancellationToken cancellationToken)
    {
        var items = new List<ImportListItem>();
        var offset = 0;
        const int limit = 50; // OL max per page
        int totalCount;

        do
        {
            var url = $"{Settings.BaseUrl}/people/{Settings.Username}/books/{shelf}.json?limit={limit}&offset={offset}";

            var response = await client.GetAsync(url, cancellationToken).ConfigureAwait(false);

            if (response.StatusCode == System.Net.HttpStatusCode.NotFound)
            {
                Logger.LogWarning("OpenLibrary user not found or shelf empty: {Username}/{Shelf}",
                    Settings.Username, shelf);
                return items;
            }

            response.EnsureSuccessStatusCode();
            var json = await response.Content.ReadAsStringAsync(cancellationToken).ConfigureAwait(false);
            var logResponse = JsonSerializer.Deserialize<OpenLibraryReadingLogResponse>(json, JsonOptions);

            if (logResponse == null || logResponse.ReadingLogEntries.Count == 0)
                break;

            totalCount = logResponse.Page.TotalCount;

            foreach (var entry in logResponse.ReadingLogEntries)
            {
                var item = MapToImportListItem(entry);
                if (item != null)
                {
                    items.Add(item);
                }
            }

            offset += logResponse.ReadingLogEntries.Count;

            // Polite delay — OpenLibrary is a nonprofit
            await Task.Delay(250, cancellationToken).ConfigureAwait(false);

            // Safety cap
            if (offset >= 5000)
            {
                Logger.LogWarning("Hit pagination limit (5,000 items) for shelf {Shelf}", shelf);
                break;
            }
        }
        while (offset < totalCount);

        return items;
    }

    private ImportListItem? MapToImportListItem(OpenLibraryReadingLogEntry entry)
    {
        var work = entry.Work;

        if (string.IsNullOrEmpty(work.Title))
            return null;

        // Extract OL work ID from key ("/works/OL45883W" → "OL45883W")
        var workId = work.Key.Replace("/works/", "");

        // Pick best ISBN — prefer ISBN-13 (length 13)
        var isbn = work.Isbns
            .Where(i => !string.IsNullOrWhiteSpace(i))
            .OrderByDescending(i => i.Length) // ISBN-13 before ISBN-10
            .FirstOrDefault();

        var item = new ImportListItem
        {
            ListId = Definition.Id,
            MediaType = MediaType.Book,
            Title = work.Title,
            Author = work.AuthorNames.FirstOrDefault() ?? string.Empty,
            Isbn = isbn,
            Year = work.FirstPublishYear ?? 0,
            WatchedAt = ParseLogDate(entry.LoggedDate)
        };

        // Store OL work ID in ImportSource for cross-referencing
        // The standard ImportListItem doesn't have an OpenLibrary field,
        // so we encode it in ImportSource alongside shelf info
        item.ImportSource = $"openlibrary:work:{workId}";

        return item;
    }

    private static DateTime? ParseLogDate(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
            return null;

        // OpenLibrary dates: "2024/01/15, 20:30:00" or "2024/01/15"
        var formats = new[]
        {
            "yyyy/MM/dd, HH:mm:ss",
            "yyyy/MM/dd",
            "yyyy-MM-dd"
        };

        if (DateTime.TryParseExact(value.Trim(), formats, CultureInfo.InvariantCulture,
            DateTimeStyles.AdjustToUniversal, out var date))
        {
            return date;
        }

        // Fallback to general parse
        if (DateTime.TryParse(value.Trim(), CultureInfo.InvariantCulture,
            DateTimeStyles.AdjustToUniversal, out date))
        {
            return date;
        }

        return null;
    }

    private static List<ImportListItem> DeduplicateByWorkKey(List<ImportListItem> items)
    {
        var seen = new HashSet<string>();
        var result = new List<ImportListItem>();

        foreach (var item in items)
        {
            // Use ImportSource which contains the work key
            var key = item.ImportSource ?? item.Title;
            if (seen.Add(key))
            {
                result.Add(item);
            }
        }

        return result;
    }

    private HttpClient CreateClient()
    {
        var client = _httpClientFactory.CreateClient();
        client.DefaultRequestHeaders.UserAgent.Add(
            new ProductInfoHeaderValue("Mouseion", "1.0"));
        client.DefaultRequestHeaders.UserAgent.Add(
            new ProductInfoHeaderValue("(+https://github.com/forkwright/mouseion)"));
        return client;
    }
}

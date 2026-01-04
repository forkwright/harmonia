// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ServiceModel.Syndication;
using System.Xml;
using Microsoft.Extensions.Logging;
using Mouseion.Common.Http;

namespace Mouseion.Core.ImportLists.RSS;

public class RssImport : ImportListBase<RssImportSettings>
{
    private readonly IHttpClient _httpClient;

    public RssImport(
        IHttpClient httpClient,
        ILogger<RssImport> logger)
        : base(logger)
    {
        _httpClient = httpClient;
    }

    public override string Name => "RSS Feed Import";
    public override ImportListType ListType => ImportListType.RSS;
    public override TimeSpan MinRefreshInterval => TimeSpan.FromHours(1);
    public override bool Enabled => true;
    public override bool EnableAuto => false;

    public override async Task<ImportListFetchResult> FetchAsync(CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(Settings.FeedUrl))
        {
            Logger.LogWarning("RSS feed URL not configured");
            return new ImportListFetchResult { AnyFailure = true };
        }

        try
        {
            Logger.LogInformation("Fetching RSS feed from {Url}", Settings.FeedUrl);

            var request = new HttpRequestBuilder(Settings.FeedUrl).Build();
            var response = await _httpClient.GetAsync(request);

            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                Logger.LogWarning("RSS feed returned status {StatusCode}", response.StatusCode);
                return new ImportListFetchResult { AnyFailure = true };
            }

            var items = ParseRssFeed(response.Content);

            return new ImportListFetchResult
            {
                Items = CleanupListItems(items),
                SyncedLists = 1
            };
        }
        catch (Exception ex)
        {
            Logger.LogError(ex, "Error fetching RSS feed from {Url}", Settings.FeedUrl);
            return new ImportListFetchResult { AnyFailure = true };
        }
    }

    private List<ImportListItem> ParseRssFeed(string content)
    {
        var items = new List<ImportListItem>();

        try
        {
            using var stringReader = new StringReader(content);
            using var xmlReader = XmlReader.Create(stringReader);

            var feed = SyndicationFeed.Load(xmlReader);

            foreach (var feedItem in feed.Items)
            {
                var title = feedItem.Title?.Text ?? string.Empty;
                if (string.IsNullOrWhiteSpace(title))
                {
                    continue;
                }

                // Basic title parsing (Title (Year) format)
                var year = 0;
                var cleanTitle = title;

                var yearMatch = System.Text.RegularExpressions.Regex.Match(title, @"\((\d{4})\)");
                if (yearMatch.Success)
                {
                    year = int.Parse(yearMatch.Groups[1].Value);
                    cleanTitle = title.Replace(yearMatch.Value, string.Empty).Trim();
                }

                items.Add(new ImportListItem
                {
                    MediaType = Settings.MediaType,
                    Title = cleanTitle,
                    Year = year
                });
            }

            Logger.LogInformation("Parsed {Count} items from RSS feed", items.Count);
        }
        catch (Exception ex)
        {
            Logger.LogError(ex, "Error parsing RSS feed");
        }

        return items;
    }
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net.Http;
using System.Net.Http.Headers;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.ImportLists.ListenBrainz;

/// <summary>
/// Imports music data from ListenBrainz — the open-source alternative to Last.fm.
/// ListenBrainz has native MusicBrainz IDs on every listen, making it the cleanest
/// import source for music data.
/// Docs: https://listenbrainz.readthedocs.io/en/latest/users/api/
/// Rate limit: No documented hard limit, but be respectful.
/// </summary>
public class ListenBrainzImportList : ImportListBase<ListenBrainzSettings>
{
    private readonly IHttpClientFactory _httpClientFactory;
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true
    };

    public ListenBrainzImportList(ILogger<ListenBrainzImportList> logger, IHttpClientFactory httpClientFactory)
        : base(logger)
    {
        _httpClientFactory = httpClientFactory;
    }

    public override string Name => "ListenBrainz";
    public override ImportListType ListType => ImportListType.ListenBrainz;
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
                Logger.LogWarning("ListenBrainz username not configured");
                result.AnyFailure = true;
                return result;
            }

            var client = CreateClient();
            var sourceCount = 0;

            if (Settings.ImportListens)
            {
                var listens = await FetchListensAsync(client, cancellationToken).ConfigureAwait(false);
                allItems.AddRange(listens);
                sourceCount++;
            }

            if (Settings.ImportFeedback)
            {
                var feedback = await FetchFeedbackAsync(client, cancellationToken).ConfigureAwait(false);
                allItems.AddRange(feedback);
                sourceCount++;
            }

            // Deduplicate by MusicBrainz recording ID or artist+track
            result.Items = DeduplicateItems(allItems);
            result.SyncedLists = sourceCount;

            Logger.LogInformation(
                "ListenBrainz import fetched {Count} unique items from {SourceCount} sources for user {Username}",
                result.Items.Count, sourceCount, Settings.Username);
        }
        catch (Exception ex) when (ex is not OperationCanceledException)
        {
            Logger.LogError(ex, "Failed to complete ListenBrainz import");
            result.AnyFailure = true;
        }

        return result;
    }

    #region Fetch Methods

    private async Task<List<ImportListItem>> FetchListensAsync(HttpClient client, CancellationToken ct)
    {
        var items = new List<ImportListItem>();
        long? maxTs = null; // For pagination — each response's oldest_listen_ts becomes next max_ts
        var batchCount = 0;
        const int maxBatches = 100; // Safety cap: 100 * 100 = 10,000 listens

        while (items.Count < Settings.MaxListens && batchCount < maxBatches)
        {
            var url = $"{Settings.BaseUrl}/user/{Settings.Username}/listens?count=100";

            if (maxTs.HasValue)
            {
                url += $"&max_ts={maxTs.Value}";
            }

            // Incremental sync
            if (Settings.LastSyncedTimestamp.HasValue)
            {
                url += $"&min_ts={Settings.LastSyncedTimestamp.Value}";
            }

            var response = await client.GetAsync(url, ct).ConfigureAwait(false);

            if (response.StatusCode == System.Net.HttpStatusCode.NotFound)
            {
                Logger.LogWarning("ListenBrainz user not found: {Username}", Settings.Username);
                return items;
            }

            response.EnsureSuccessStatusCode();
            var json = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
            var data = JsonSerializer.Deserialize<ListenBrainzListensResponse>(json, JsonOptions);

            if (data?.Payload.Listens == null || data.Payload.Listens.Count == 0)
                break;

            foreach (var listen in data.Payload.Listens)
            {
                var item = MapListenToItem(listen);
                if (item != null)
                {
                    items.Add(item);
                }
            }

            // Paginate backwards in time
            maxTs = data.Payload.OldestListenTs;
            if (!maxTs.HasValue || data.Payload.Listens.Count < 100)
                break; // No more data

            batchCount++;
            await Task.Delay(250, ct).ConfigureAwait(false); // Polite delay
        }

        return items;
    }

    private async Task<List<ImportListItem>> FetchFeedbackAsync(HttpClient client, CancellationToken ct)
    {
        var items = new List<ImportListItem>();
        var offset = 0;
        const int limit = 100;

        while (true)
        {
            var url = $"{Settings.BaseUrl}/feedback/user/{Settings.Username}/get-feedback?offset={offset}&count={limit}";

            var response = await client.GetAsync(url, ct).ConfigureAwait(false);
            response.EnsureSuccessStatusCode();
            var json = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
            var data = JsonSerializer.Deserialize<ListenBrainzFeedbackResponse>(json, JsonOptions);

            if (data?.Feedback == null || data.Feedback.Count == 0)
                break;

            foreach (var feedback in data.Feedback)
            {
                var item = MapFeedbackToItem(feedback);
                if (item != null)
                {
                    items.Add(item);
                }
            }

            offset += data.Feedback.Count;

            if (offset >= data.TotalCount)
                break;

            // Safety cap
            if (offset >= 5000)
            {
                Logger.LogWarning("Hit feedback pagination limit (5,000 items)");
                break;
            }

            await Task.Delay(250, ct).ConfigureAwait(false);
        }

        return items;
    }

    #endregion

    #region Mapping

    private ImportListItem? MapListenToItem(ListenBrainzListen listen)
    {
        var metadata = listen.TrackMetadata;
        if (string.IsNullOrEmpty(metadata.TrackName))
            return null;

        var item = new ImportListItem
        {
            ListId = Definition.Id,
            MediaType = MediaType.Music,
            Title = metadata.TrackName,
            Artist = metadata.ArtistName,
            Album = metadata.ReleaseName,
            ImportSource = "listenbrainz:listen",
            WatchedAt = DateTimeOffset.FromUnixTimeSeconds(listen.ListenedAt).UtcDateTime
        };

        // MusicBrainz ID resolution — prefer MBID mapping (server-side), then additional_info (client-submitted)
        var releaseMbid = metadata.MbidMapping?.ReleaseMbid
            ?? metadata.AdditionalInfo?.ReleaseMbid;
        var recordingMbid = metadata.MbidMapping?.RecordingMbid
            ?? metadata.AdditionalInfo?.RecordingMbid;
        var releaseGroupMbid = metadata.AdditionalInfo?.ReleaseGroupMbid;

        // Use release MBID for album matching (most useful for Mouseion),
        // fall back to recording MBID, then release group
        if (TryParseMbid(releaseMbid, out var mbid))
        {
            item.MusicBrainzId = mbid;
        }
        else if (TryParseMbid(releaseGroupMbid, out mbid))
        {
            item.MusicBrainzId = mbid;
        }
        else if (TryParseMbid(recordingMbid, out mbid))
        {
            item.MusicBrainzId = mbid;
        }

        return item;
    }

    private ImportListItem? MapFeedbackToItem(ListenBrainzFeedback feedback)
    {
        // Need either a recording MBID or track metadata
        var hasMetadata = feedback.TrackMetadata != null
            && !string.IsNullOrEmpty(feedback.TrackMetadata.TrackName);
        var hasMbid = !string.IsNullOrEmpty(feedback.RecordingMbid);

        if (!hasMetadata && !hasMbid)
            return null;

        var item = new ImportListItem
        {
            ListId = Definition.Id,
            MediaType = MediaType.Music,
            Title = feedback.TrackMetadata?.TrackName ?? string.Empty,
            Artist = feedback.TrackMetadata?.ArtistName ?? string.Empty,
            Album = feedback.TrackMetadata?.ReleaseName,
            ImportSource = feedback.Score > 0 ? "listenbrainz:love" : "listenbrainz:hate",
            // Love → 10, Hate → 1
            UserRating = feedback.Score > 0 ? 10 : (feedback.Score < 0 ? 1 : null),
            WatchedAt = feedback.Created > 0
                ? DateTimeOffset.FromUnixTimeSeconds(feedback.Created).UtcDateTime
                : null
        };

        if (TryParseMbid(feedback.RecordingMbid, out var mbid))
        {
            item.MusicBrainzId = mbid;
        }

        return item;
    }

    #endregion

    #region Helpers

    private HttpClient CreateClient()
    {
        var client = _httpClientFactory.CreateClient();
        client.DefaultRequestHeaders.UserAgent.Add(
            new ProductInfoHeaderValue("Mouseion", "1.0"));

        // Token auth for endpoints that require it
        if (!string.IsNullOrEmpty(Settings.UserToken))
        {
            client.DefaultRequestHeaders.Authorization =
                new AuthenticationHeaderValue("Token", Settings.UserToken);
        }

        return client;
    }

    private static bool TryParseMbid(string? mbid, out Guid result)
    {
        result = Guid.Empty;
        if (string.IsNullOrWhiteSpace(mbid))
            return false;

        return Guid.TryParse(mbid, out result) && result != Guid.Empty;
    }

    private static List<ImportListItem> DeduplicateItems(List<ImportListItem> items)
    {
        var seen = new HashSet<string>();
        var result = new List<ImportListItem>();

        foreach (var item in items)
        {
            // MusicBrainz ID is authoritative when available
            var key = item.MusicBrainzId != Guid.Empty
                ? $"mbid:{item.MusicBrainzId}"
                : $"text:{item.Artist?.ToLowerInvariant()}:{item.Title.ToLowerInvariant()}";

            if (seen.Add(key))
            {
                result.Add(item);
            }
        }

        return result;
    }

    #endregion
}

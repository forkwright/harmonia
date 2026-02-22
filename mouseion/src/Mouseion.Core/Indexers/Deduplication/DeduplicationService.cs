// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Download;
using Serilog;

namespace Mouseion.Core.Indexers.Deduplication;

public interface IDeduplicationService
{
    Task<SearchDecision> ShouldSearchAsync(int mediaItemId, string indexerName, string mediaType, CancellationToken ct = default);
    Task RecordSearchAsync(SearchHistoryEntry entry, CancellationToken ct = default);
    Task<GrabDecision> ShouldGrabAsync(string releaseGuid, int mediaItemId, CancellationToken ct = default);
    Task RecordGrabAsync(GrabbedRelease grab, CancellationToken ct = default);
    Task SkipReleaseAsync(int mediaItemId, string releaseGuid, string releaseTitle, string reason, CancellationToken ct = default);
    Task<bool> IsInDownloadQueueAsync(int mediaItemId, CancellationToken ct = default);
    Task<List<T>> FilterReleasesAsync<T>(int mediaItemId, IEnumerable<T> releases, Func<T, string> guidSelector, CancellationToken ct = default);
    Task CleanupAsync(int retentionDays = 90, CancellationToken ct = default);
}

public class SearchDecision
{
    public bool ShouldSearch { get; set; }
    public string? SkipReason { get; set; }
    public DateTime? LastSearchedAt { get; set; }
    public static SearchDecision Allow() => new() { ShouldSearch = true };
    public static SearchDecision Skip(string reason, DateTime? lastSearched = null) => new() { ShouldSearch = false, SkipReason = reason, LastSearchedAt = lastSearched };
}

public class GrabDecision
{
    public bool ShouldGrab { get; set; }
    public string? SkipReason { get; set; }
    public static GrabDecision Allow() => new() { ShouldGrab = true };
    public static GrabDecision Skip(string reason) => new() { ShouldGrab = false, SkipReason = reason };
}

public class DeduplicationService : IDeduplicationService
{
    private readonly ISearchHistoryRepository _searchHistoryRepo;
    private readonly IGrabbedReleaseRepository _grabbedRepo;
    private readonly ISkippedReleaseRepository _skippedRepo;
    private readonly IEnumerable<IDownloadClient> _downloadClients;
    private static readonly ILogger Logger = Log.ForContext<DeduplicationService>();

    private static readonly Dictionary<string, TimeSpan> SearchCooldowns = new(StringComparer.OrdinalIgnoreCase)
    {
        ["Movie"] = TimeSpan.FromHours(12),
        ["TV"] = TimeSpan.FromHours(6),
        ["Music"] = TimeSpan.FromHours(24),
        ["Book"] = TimeSpan.FromDays(3),
        ["Audiobook"] = TimeSpan.FromDays(3),
        ["Manga"] = TimeSpan.FromHours(12),
        ["Comic"] = TimeSpan.FromDays(1),
        ["Podcast"] = TimeSpan.FromHours(2),
    };

    public DeduplicationService(
        ISearchHistoryRepository searchHistoryRepo,
        IGrabbedReleaseRepository grabbedRepo,
        ISkippedReleaseRepository skippedRepo,
        IEnumerable<IDownloadClient> downloadClients)
    {
        _searchHistoryRepo = searchHistoryRepo;
        _grabbedRepo = grabbedRepo;
        _skippedRepo = skippedRepo;
        _downloadClients = downloadClients;
    }

    public async Task<SearchDecision> ShouldSearchAsync(int mediaItemId, string indexerName, string mediaType, CancellationToken ct = default)
    {
        if (await IsInDownloadQueueAsync(mediaItemId, ct).ConfigureAwait(false))
            return SearchDecision.Skip("Already in download queue");

        var lastSearch = await _searchHistoryRepo.GetLastSearchAsync(mediaItemId, indexerName, ct).ConfigureAwait(false);
        if (lastSearch != null)
        {
            var cooldown = SearchCooldowns.GetValueOrDefault(mediaType, TimeSpan.FromHours(12));
            if (DateTime.UtcNow < lastSearch.SearchedAt + cooldown)
                return SearchDecision.Skip($"Searched {(DateTime.UtcNow - lastSearch.SearchedAt).TotalHours:F1}h ago, cooldown is {cooldown.TotalHours:F0}h", lastSearch.SearchedAt);
        }

        return SearchDecision.Allow();
    }

    public async Task RecordSearchAsync(SearchHistoryEntry entry, CancellationToken ct = default)
    {
        entry.SearchedAt = DateTime.UtcNow;
        await _searchHistoryRepo.InsertAsync(entry, ct).ConfigureAwait(false);
    }

    public async Task<GrabDecision> ShouldGrabAsync(string releaseGuid, int mediaItemId, CancellationToken ct = default)
    {
        if (await _grabbedRepo.IsGrabbedAsync(releaseGuid, ct).ConfigureAwait(false))
            return GrabDecision.Skip("Already grabbed");
        if (await _skippedRepo.IsSkippedAsync(releaseGuid, ct).ConfigureAwait(false))
            return GrabDecision.Skip("Previously skipped/rejected");
        return GrabDecision.Allow();
    }

    public async Task RecordGrabAsync(GrabbedRelease grab, CancellationToken ct = default)
    {
        grab.GrabbedAt = DateTime.UtcNow;
        await _grabbedRepo.InsertAsync(grab, ct).ConfigureAwait(false);
        Logger.Information("Recorded grab: {Title} from {Indexer}", grab.ReleaseTitle, grab.IndexerName);
    }

    public async Task SkipReleaseAsync(int mediaItemId, string releaseGuid, string releaseTitle, string reason, CancellationToken ct = default)
    {
        await _skippedRepo.InsertAsync(new SkippedRelease
        {
            MediaItemId = mediaItemId, ReleaseGuid = releaseGuid,
            ReleaseTitle = releaseTitle, Reason = reason, SkippedAt = DateTime.UtcNow
        }, ct).ConfigureAwait(false);
    }

    public async Task<bool> IsInDownloadQueueAsync(int mediaItemId, CancellationToken ct = default)
    {
        var grabbed = await _grabbedRepo.GetByMediaItemAsync(mediaItemId, ct).ConfigureAwait(false);
        if (!grabbed.Any()) return false;

        foreach (var client in _downloadClients)
        {
            try
            {
                var items = await client.GetItemsAsync(ct).ConfigureAwait(false);
                if (items.Any(i => grabbed.Any(g => g.DownloadClientId == i.DownloadId) &&
                    i.Status != DownloadItemStatus.Completed && i.Status != DownloadItemStatus.Failed))
                    return true;
            }
            catch (Exception ex) { Logger.Warning(ex, "Failed to check {Client} queue", client.Name); }
        }
        return false;
    }

    public async Task<List<T>> FilterReleasesAsync<T>(int mediaItemId, IEnumerable<T> releases, Func<T, string> guidSelector, CancellationToken ct = default)
    {
        var filtered = new List<T>();
        foreach (var release in releases)
        {
            var decision = await ShouldGrabAsync(guidSelector(release), mediaItemId, ct).ConfigureAwait(false);
            if (decision.ShouldGrab) filtered.Add(release);
        }
        return filtered;
    }

    public async Task CleanupAsync(int retentionDays = 90, CancellationToken ct = default)
    {
        await _searchHistoryRepo.CleanupOlderThanAsync(DateTime.UtcNow.AddDays(-retentionDays), ct).ConfigureAwait(false);
    }
}

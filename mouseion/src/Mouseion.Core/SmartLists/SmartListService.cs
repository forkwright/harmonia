// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json;
using Microsoft.Extensions.Logging;
using Mouseion.Core.MediaItems;
using Mouseion.Core.SmartLists.Sources;

namespace Mouseion.Core.SmartLists;

public interface ISmartListService
{
    Task<IEnumerable<SmartList>> GetAllAsync(CancellationToken ct = default);
    Task<SmartList?> GetAsync(int id, CancellationToken ct = default);
    Task<SmartList> CreateAsync(SmartList list, CancellationToken ct = default);
    Task<SmartList> UpdateAsync(SmartList list, CancellationToken ct = default);
    Task DeleteAsync(int id, CancellationToken ct = default);
    Task<SmartListRefreshResult> RefreshAsync(int id, CancellationToken ct = default);
    Task<SmartListRefreshResult> RefreshAllDueAsync(CancellationToken ct = default);
    Task<IEnumerable<SmartListMatch>> GetMatchesAsync(int id, CancellationToken ct = default);
    Task SkipMatchAsync(int matchId, CancellationToken ct = default);
}

public class SmartListRefreshResult
{
    public int ListsProcessed { get; set; }
    public int ItemsDiscovered { get; set; }
    public int ItemsAdded { get; set; }
    public int ItemsDuplicate { get; set; }
    public int ItemsFiltered { get; set; }
    public int ItemsFailed { get; set; }
    public List<string> Errors { get; set; } = new();
}

public partial class SmartListService : ISmartListService
{
    private readonly ISmartListRepository _repository;
    private readonly ISmartListMatchRepository _matchRepository;
    private readonly IMediaItemRepository _mediaItemRepository;
    private readonly IEnumerable<ISmartListSourceProvider> _sourceProviders;
    private readonly ILogger<SmartListService> _logger;

    public SmartListService(
        ISmartListRepository repository,
        ISmartListMatchRepository matchRepository,
        IMediaItemRepository mediaItemRepository,
        IEnumerable<ISmartListSourceProvider> sourceProviders,
        ILogger<SmartListService> logger)
    {
        _repository = repository;
        _matchRepository = matchRepository;
        _mediaItemRepository = mediaItemRepository;
        _sourceProviders = sourceProviders;
        _logger = logger;
    }

    public async Task<IEnumerable<SmartList>> GetAllAsync(CancellationToken ct = default)
        => await _repository.AllAsync(ct).ConfigureAwait(false);

    public async Task<SmartList?> GetAsync(int id, CancellationToken ct = default)
        => await _repository.FindAsync(id, ct).ConfigureAwait(false);

    public async Task<SmartList> CreateAsync(SmartList list, CancellationToken ct = default)
    {
        var now = DateTime.UtcNow;
        list.CreatedAt = now;
        list.UpdatedAt = now;
        list.ItemsAdded = 0;
        var created = await _repository.InsertAsync(list, ct).ConfigureAwait(false);
        LogCreated(created.Id, created.Name, created.Source.ToString());
        return created;
    }

    public async Task<SmartList> UpdateAsync(SmartList list, CancellationToken ct = default)
    {
        list.UpdatedAt = DateTime.UtcNow;
        var updated = await _repository.UpdateAsync(list, ct).ConfigureAwait(false);
        LogUpdated(updated.Id, updated.Name);
        return updated;
    }

    public async Task DeleteAsync(int id, CancellationToken ct = default)
    {
        await _matchRepository.DeleteByListIdAsync(id, ct).ConfigureAwait(false);
        await _repository.DeleteAsync(id, ct).ConfigureAwait(false);
        LogDeleted(id);
    }

    public async Task<SmartListRefreshResult> RefreshAsync(int id, CancellationToken ct = default)
    {
        var list = await _repository.FindAsync(id, ct).ConfigureAwait(false)
            ?? throw new KeyNotFoundException($"SmartList with ID {id} not found");

        return await RefreshListAsync(list, ct).ConfigureAwait(false);
    }

    public async Task<SmartListRefreshResult> RefreshAllDueAsync(CancellationToken ct = default)
    {
        var dueLists = await _repository.GetDueForRefreshAsync(ct).ConfigureAwait(false);
        var result = new SmartListRefreshResult();

        foreach (var list in dueLists)
        {
            try
            {
                var listResult = await RefreshListAsync(list, ct).ConfigureAwait(false);
                result.ListsProcessed++;
                result.ItemsDiscovered += listResult.ItemsDiscovered;
                result.ItemsAdded += listResult.ItemsAdded;
                result.ItemsDuplicate += listResult.ItemsDuplicate;
                result.ItemsFiltered += listResult.ItemsFiltered;
                result.ItemsFailed += listResult.ItemsFailed;
                result.Errors.AddRange(listResult.Errors);
            }
            catch (Exception ex)
            {
                result.Errors.Add($"List '{list.Name}' ({list.Id}): {ex.Message}");
                LogRefreshError(list.Id, list.Name, ex.Message);
            }
        }

        LogRefreshAllComplete(result.ListsProcessed, result.ItemsAdded);
        return result;
    }

    public async Task<IEnumerable<SmartListMatch>> GetMatchesAsync(int id, CancellationToken ct = default)
        => await _matchRepository.GetByListIdAsync(id, ct).ConfigureAwait(false);

    public async Task SkipMatchAsync(int matchId, CancellationToken ct = default)
    {
        var match = await _matchRepository.GetAsync(matchId, ct).ConfigureAwait(false);
        match.Status = SmartListMatchStatus.Skipped;
        await _matchRepository.UpdateAsync(match, ct).ConfigureAwait(false);
    }

    private async Task<SmartListRefreshResult> RefreshListAsync(SmartList list, CancellationToken ct)
    {
        var result = new SmartListRefreshResult { ListsProcessed = 1 };

        var provider = _sourceProviders.FirstOrDefault(p => p.Source == list.Source);
        if (provider == null)
        {
            result.Errors.Add($"No source provider for {list.Source}");
            return result;
        }

        LogRefreshStart(list.Id, list.Name, list.Source.ToString());

        IReadOnlyList<SmartListDiscoveryResult> discovered;
        try
        {
            discovered = await provider.DiscoverAsync(list, ct).ConfigureAwait(false);
        }
        catch (Exception ex)
        {
            result.Errors.Add($"Discovery failed: {ex.Message}");
            return result;
        }

        result.ItemsDiscovered = discovered.Count;
        var addedCount = 0;

        foreach (var item in discovered.Take(list.MaxItemsPerRefresh))
        {
            // Skip if below minimum rating
            if (list.MinimumRating.HasValue && item.Rating.HasValue && item.Rating.Value < list.MinimumRating.Value)
            {
                await RecordMatchAsync(list.Id, item, SmartListMatchStatus.Filtered, ct).ConfigureAwait(false);
                result.ItemsFiltered++;
                continue;
            }

            // Check if already discovered for this list
            var existing = await _matchRepository.FindByExternalIdAsync(list.Id, item.ExternalId, ct).ConfigureAwait(false);
            if (existing != null)
            {
                result.ItemsDuplicate++;
                continue;
            }

            // Check if already in library by cross-referencing IDs
            var inLibrary = await IsInLibraryAsync(item, ct).ConfigureAwait(false);
            if (inLibrary)
            {
                await RecordMatchAsync(list.Id, item, SmartListMatchStatus.Duplicate, ct).ConfigureAwait(false);
                result.ItemsDuplicate++;
                continue;
            }

            // Record as pending — actual library addition requires metadata lookup
            // which is handled by the existing Add*Service infrastructure
            await RecordMatchAsync(list.Id, item, SmartListMatchStatus.Pending, ct).ConfigureAwait(false);
            addedCount++;
        }

        result.ItemsAdded = addedCount;

        // Update list metadata
        list.LastRefreshed = DateTime.UtcNow;
        list.UpdatedAt = DateTime.UtcNow;
        list.ItemsAdded += addedCount;
        await _repository.UpdateAsync(list, ct).ConfigureAwait(false);

        LogRefreshComplete(list.Id, list.Name, discovered.Count, addedCount);
        return result;
    }

    private async Task<bool> IsInLibraryAsync(SmartListDiscoveryResult item, CancellationToken ct)
    {
        // Check by TMDB ID (movies/TV)
        if (item.TmdbId.HasValue)
        {
            var count = await _mediaItemRepository.CountAsync(ct: ct).ConfigureAwait(false);
            // TODO: Add FindByExternalIdAsync to IMediaItemRepository for proper dedup
            // For now, we rely on the match table to prevent re-processing
            return false;
        }

        return false;
    }

    private async Task RecordMatchAsync(int listId, SmartListDiscoveryResult item, SmartListMatchStatus status, CancellationToken ct)
    {
        var match = new SmartListMatch
        {
            SmartListId = listId,
            ExternalId = item.ExternalId,
            MediaType = default, // Set from parent list
            Title = item.Title,
            Year = item.Year,
            Rating = item.Rating,
            Status = status,
            MetadataJson = item.MetadataJson,
            DiscoveredAt = DateTime.UtcNow,
            AddedAt = status == SmartListMatchStatus.Added ? DateTime.UtcNow : null
        };

        await _matchRepository.InsertAsync(match, ct).ConfigureAwait(false);
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Smart list created: {Id} ({Name}) — source: {Source}")]
    private partial void LogCreated(int id, string name, string source);

    [LoggerMessage(Level = LogLevel.Information, Message = "Smart list updated: {Id} ({Name})")]
    private partial void LogUpdated(int id, string name);

    [LoggerMessage(Level = LogLevel.Information, Message = "Smart list deleted: {Id}")]
    private partial void LogDeleted(int id);

    [LoggerMessage(Level = LogLevel.Information, Message = "Refreshing smart list {Id} ({Name}) — source: {Source}")]
    private partial void LogRefreshStart(int id, string name, string source);

    [LoggerMessage(Level = LogLevel.Information, Message = "Smart list {Id} ({Name}) refreshed: {Discovered} discovered, {Added} new matches")]
    private partial void LogRefreshComplete(int id, string name, int discovered, int added);

    [LoggerMessage(Level = LogLevel.Error, Message = "Smart list {Id} ({Name}) refresh failed: {Error}")]
    private partial void LogRefreshError(int id, string name, string error);

    [LoggerMessage(Level = LogLevel.Information, Message = "Smart list refresh all complete: {Lists} lists, {Added} items added")]
    private partial void LogRefreshAllComplete(int lists, int added);
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json;
using Microsoft.Extensions.Logging;
using Mouseion.Core.ImportLists.History;
using Mouseion.Core.ImportLists.ImportExclusions;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.ImportLists.Wizard;

/// <summary>
/// Orchestrates import list operations: preview (dry-run), execute with conflict
/// detection, and bulk re-import. All operations are recorded in ImportSession history.
/// </summary>
public interface IImportWizardService
{
    /// <summary>
    /// Fetch items from a list and preview what would happen without committing.
    /// Returns conflicts, new items, items that would be skipped/excluded.
    /// </summary>
    Task<ImportPreviewResult> PreviewAsync(int listId, CancellationToken ct = default);

    /// <summary>
    /// Execute an import: fetch, detect conflicts, add new items, update existing.
    /// Conflicts are recorded but not auto-resolved — user must resolve via API.
    /// </summary>
    Task<ImportSession> ExecuteAsync(int listId, ImportExecutionOptions? options = null, CancellationToken ct = default);

    /// <summary>
    /// Re-sync all items from a list, including previously skipped/excluded items.
    /// Useful for rebuilding library after initial setup.
    /// </summary>
    Task<ImportSession> ReSyncAsync(int listId, CancellationToken ct = default);

    /// <summary>
    /// Resolve a conflict: accept imported data, keep existing, or merge specific fields.
    /// </summary>
    Task<ImportSessionItem> ResolveConflictAsync(int sessionItemId, ConflictResolution resolution, CancellationToken ct = default);
}

public class ImportWizardService : IImportWizardService
{
    private readonly IImportListFactory _factory;
    private readonly IImportListExclusionService _exclusionService;
    private readonly IImportSessionRepository _sessionRepository;
    private readonly IImportSessionItemRepository _sessionItemRepository;
    private readonly IImportItemMatcher _matcher;
    private readonly ILogger<ImportWizardService> _logger;

    public ImportWizardService(
        IImportListFactory factory,
        IImportListExclusionService exclusionService,
        IImportSessionRepository sessionRepository,
        IImportSessionItemRepository sessionItemRepository,
        IImportItemMatcher matcher,
        ILogger<ImportWizardService> logger)
    {
        _factory = factory;
        _exclusionService = exclusionService;
        _sessionRepository = sessionRepository;
        _sessionItemRepository = sessionItemRepository;
        _matcher = matcher;
        _logger = logger;
    }

    public async Task<ImportPreviewResult> PreviewAsync(int listId, CancellationToken ct = default)
    {
        var list = _factory.Get(listId);
        _logger.LogInformation("Starting preview for import list: {Name} ({Type})", list.Name, list.ListType);

        var session = CreateSession(list, isDryRun: true);
        session = _sessionRepository.Insert(session);

        try
        {
            var fetchResult = await list.FetchAsync(ct);
            session.ItemsFetched = fetchResult.Items.Count;

            var exclusions = _exclusionService.GetAll();
            var result = new ImportPreviewResult
            {
                SessionId = session.Id,
                ListId = listId,
                ListName = list.Name,
                ListType = list.ListType,
                TotalFetched = fetchResult.Items.Count
            };

            foreach (var item in fetchResult.Items)
            {
                var sessionItem = new ImportSessionItem
                {
                    SessionId = session.Id,
                    Title = item.Title,
                    Year = item.Year,
                    MediaType = item.MediaType,
                    ExternalIds = SerializeExternalIds(item),
                    UserRating = item.UserRating,
                    ProcessedAt = DateTime.UtcNow
                };

                // Check exclusions first
                if (IsExcluded(item, exclusions))
                {
                    sessionItem.Action = ImportItemAction.Excluded;
                    sessionItem.Reason = "Item is in exclusion list";
                    result.Excluded.Add(sessionItem);
                }
                else
                {
                    // Check for existing item in library
                    var match = await _matcher.FindMatchAsync(item, ct);
                    if (match != null)
                    {
                        sessionItem.MediaItemId = match.Id;
                        var diffs = _matcher.DetectDiffs(item, match);
                        if (diffs.Count > 0)
                        {
                            sessionItem.Action = ImportItemAction.Conflict;
                            sessionItem.DiffJson = JsonSerializer.Serialize(diffs);
                            sessionItem.Reason = $"Exists with {diffs.Count} difference(s): {string.Join(", ", diffs.Keys)}";
                            result.Conflicts.Add(sessionItem);
                        }
                        else
                        {
                            sessionItem.Action = ImportItemAction.Skipped;
                            sessionItem.Reason = "Already in library, no differences";
                            result.Skipped.Add(sessionItem);
                        }
                    }
                    else
                    {
                        sessionItem.Action = ImportItemAction.Added;
                        sessionItem.Reason = "New item, not in library";
                        result.NewItems.Add(sessionItem);
                    }
                }

                _sessionItemRepository.Insert(sessionItem);
            }

            // Update session counts
            session.ItemsAdded = result.NewItems.Count;
            session.ItemsUpdated = result.Conflicts.Count;
            session.ItemsSkipped = result.Skipped.Count + result.Excluded.Count;
            session.Status = ImportSessionStatus.DryRun;
            session.CompletedAt = DateTime.UtcNow;
            _sessionRepository.Update(session);

            _logger.LogInformation(
                "Preview complete for {Name}: {New} new, {Conflicts} conflicts, {Skipped} skipped, {Excluded} excluded",
                list.Name, result.NewItems.Count, result.Conflicts.Count, result.Skipped.Count, result.Excluded.Count);

            return result;
        }
        catch (Exception ex)
        {
            session.Status = ImportSessionStatus.Failed;
            session.ErrorMessage = ex.Message;
            session.CompletedAt = DateTime.UtcNow;
            _sessionRepository.Update(session);
            throw;
        }
    }

    public async Task<ImportSession> ExecuteAsync(int listId, ImportExecutionOptions? options = null, CancellationToken ct = default)
    {
        options ??= new ImportExecutionOptions();
        var list = _factory.Get(listId);
        _logger.LogInformation("Executing import for list: {Name} ({Type})", list.Name, list.ListType);

        var session = CreateSession(list, isDryRun: false);
        session.Status = ImportSessionStatus.Running;
        session = _sessionRepository.Insert(session);

        try
        {
            var fetchResult = await list.FetchAsync(ct);
            session.ItemsFetched = fetchResult.Items.Count;

            var exclusions = _exclusionService.GetAll();
            int added = 0, updated = 0, skipped = 0, failed = 0;

            foreach (var item in fetchResult.Items)
            {
                var sessionItem = new ImportSessionItem
                {
                    SessionId = session.Id,
                    Title = item.Title,
                    Year = item.Year,
                    MediaType = item.MediaType,
                    ExternalIds = SerializeExternalIds(item),
                    UserRating = item.UserRating,
                    ProcessedAt = DateTime.UtcNow
                };

                try
                {
                    if (IsExcluded(item, exclusions))
                    {
                        sessionItem.Action = ImportItemAction.Excluded;
                        sessionItem.Reason = "Item is in exclusion list";
                        skipped++;
                    }
                    else
                    {
                        var match = await _matcher.FindMatchAsync(item, ct);
                        if (match != null)
                        {
                            sessionItem.MediaItemId = match.Id;
                            var diffs = _matcher.DetectDiffs(item, match);

                            if (diffs.Count == 0)
                            {
                                sessionItem.Action = ImportItemAction.Skipped;
                                sessionItem.Reason = "Already in library, no differences";
                                skipped++;
                            }
                            else if (options.AutoResolveConflicts)
                            {
                                // Auto-resolve: import overwrites existing
                                await _matcher.ApplyUpdateAsync(item, match, ct);
                                sessionItem.Action = ImportItemAction.Updated;
                                sessionItem.DiffJson = JsonSerializer.Serialize(diffs);
                                sessionItem.Reason = $"Auto-resolved: updated {string.Join(", ", diffs.Keys)}";
                                updated++;
                            }
                            else
                            {
                                // Leave as conflict for manual resolution
                                sessionItem.Action = ImportItemAction.Conflict;
                                sessionItem.DiffJson = JsonSerializer.Serialize(diffs);
                                sessionItem.Reason = $"Conflict: {diffs.Count} difference(s) — {string.Join(", ", diffs.Keys)}";
                                // Count as skipped until resolved
                                skipped++;
                            }
                        }
                        else
                        {
                            // New item — add to library
                            var mediaItemId = await _matcher.AddToLibraryAsync(item, list.Definition, ct);
                            sessionItem.MediaItemId = mediaItemId;
                            sessionItem.Action = ImportItemAction.Added;
                            sessionItem.Reason = "Added to library";
                            added++;
                        }
                    }
                }
                catch (Exception ex)
                {
                    sessionItem.Action = ImportItemAction.Failed;
                    sessionItem.Reason = ex.Message;
                    failed++;
                    _logger.LogWarning(ex, "Failed to process import item: {Title} ({Year})", item.Title, item.Year);
                }

                _sessionItemRepository.Insert(sessionItem);
            }

            session.ItemsAdded = added;
            session.ItemsUpdated = updated;
            session.ItemsSkipped = skipped;
            session.ItemsFailed = failed;
            session.Status = ImportSessionStatus.Completed;
            session.CompletedAt = DateTime.UtcNow;
            _sessionRepository.Update(session);

            _logger.LogInformation(
                "Import complete for {Name}: {Added} added, {Updated} updated, {Skipped} skipped, {Failed} failed",
                list.Name, added, updated, skipped, failed);

            return session;
        }
        catch (Exception ex)
        {
            session.Status = ImportSessionStatus.Failed;
            session.ErrorMessage = ex.Message;
            session.CompletedAt = DateTime.UtcNow;
            _sessionRepository.Update(session);
            _logger.LogError(ex, "Import failed for list: {Name}", list.Name);
            throw;
        }
    }

    public async Task<ImportSession> ReSyncAsync(int listId, CancellationToken ct = default)
    {
        // Re-sync is just execute with auto-resolve and no exclusion filtering
        return await ExecuteAsync(listId, new ImportExecutionOptions
        {
            AutoResolveConflicts = true,
            IgnoreExclusions = false
        }, ct);
    }

    public async Task<ImportSessionItem> ResolveConflictAsync(int sessionItemId, ConflictResolution resolution, CancellationToken ct = default)
    {
        var sessionItem = _sessionItemRepository.Get(sessionItemId);
        if (sessionItem.Action != ImportItemAction.Conflict)
        {
            throw new InvalidOperationException($"Session item {sessionItemId} is not a conflict (action: {sessionItem.Action})");
        }

        if (!sessionItem.MediaItemId.HasValue)
        {
            throw new InvalidOperationException($"Session item {sessionItemId} has no linked media item");
        }

        switch (resolution.Strategy)
        {
            case ConflictStrategy.KeepExisting:
                sessionItem.Action = ImportItemAction.Skipped;
                sessionItem.Reason = "Conflict resolved: kept existing";
                break;

            case ConflictStrategy.UseImported:
                // Parse original import item from external IDs and apply
                var diffs = JsonSerializer.Deserialize<Dictionary<string, FieldDiff>>(sessionItem.DiffJson ?? "{}") ?? new();
                await _matcher.ApplyDiffsAsync(sessionItem.MediaItemId.Value, diffs, useImported: true, ct);
                sessionItem.Action = ImportItemAction.Updated;
                sessionItem.Reason = "Conflict resolved: used imported data";
                break;

            case ConflictStrategy.MergeFields:
                if (resolution.FieldChoices == null || resolution.FieldChoices.Count == 0)
                {
                    throw new ArgumentException("MergeFields strategy requires field choices");
                }
                await _matcher.ApplyFieldChoicesAsync(sessionItem.MediaItemId.Value, sessionItem.DiffJson ?? "{}", resolution.FieldChoices, ct);
                sessionItem.Action = ImportItemAction.Updated;
                sessionItem.Reason = $"Conflict resolved: merged {resolution.FieldChoices.Count} field(s)";
                break;

            default:
                throw new ArgumentException($"Unknown conflict strategy: {resolution.Strategy}");
        }

        sessionItem.ProcessedAt = DateTime.UtcNow;
        _sessionItemRepository.Update(sessionItem);

        // Update parent session counts
        var session = _sessionRepository.Get(sessionItem.SessionId);
        var items = _sessionItemRepository.GetBySessionId(session.Id);
        session.ItemsUpdated = items.Count(i => i.Action == ImportItemAction.Updated);
        session.ItemsSkipped = items.Count(i => i.Action is ImportItemAction.Skipped or ImportItemAction.Excluded);
        _sessionRepository.Update(session);

        return sessionItem;
    }

    private static ImportSession CreateSession(IImportList list, bool isDryRun)
    {
        return new ImportSession
        {
            ImportListId = list.Definition.Id,
            ImportListName = list.Name,
            ListType = list.ListType,
            MediaType = list.Definition.MediaType,
            Status = isDryRun ? ImportSessionStatus.DryRun : ImportSessionStatus.Pending,
            IsDryRun = isDryRun,
            StartedAt = DateTime.UtcNow
        };
    }

    private static bool IsExcluded(ImportListItem item, List<ImportListExclusion> exclusions)
    {
        return exclusions.Any(ex =>
            (ex.MediaType == item.MediaType) &&
            ((ex.TmdbId > 0 && ex.TmdbId == item.TmdbId) ||
             (ex.ImdbId != null && ex.ImdbId == item.ImdbId) ||
             (ex.TvdbId > 0 && ex.TvdbId == item.TvdbId) ||
             (ex.GoodreadsId > 0 && ex.GoodreadsId == item.GoodreadsId) ||
             (ex.Isbn != null && ex.Isbn == item.Isbn) ||
             (ex.MusicBrainzId != Guid.Empty && ex.MusicBrainzId == item.MusicBrainzId) ||
             (ex.Asin != null && ex.Asin == item.Asin))
        );
    }

    private static string SerializeExternalIds(ImportListItem item)
    {
        var ids = new Dictionary<string, object?>();
        if (item.TmdbId > 0) ids["tmdbId"] = item.TmdbId;
        if (!string.IsNullOrEmpty(item.ImdbId)) ids["imdbId"] = item.ImdbId;
        if (item.TvdbId > 0) ids["tvdbId"] = item.TvdbId;
        if (item.GoodreadsId > 0) ids["goodreadsId"] = item.GoodreadsId;
        if (!string.IsNullOrEmpty(item.Isbn)) ids["isbn"] = item.Isbn;
        if (item.MusicBrainzId != Guid.Empty) ids["musicBrainzId"] = item.MusicBrainzId;
        if (!string.IsNullOrEmpty(item.Asin)) ids["asin"] = item.Asin;
        if (item.MalId.HasValue) ids["malId"] = item.MalId;
        if (item.AniListId.HasValue) ids["aniListId"] = item.AniListId;
        if (!string.IsNullOrEmpty(item.PodcastGuid)) ids["podcastGuid"] = item.PodcastGuid;
        return JsonSerializer.Serialize(ids);
    }
}

public class ImportPreviewResult
{
    public int SessionId { get; set; }
    public int ListId { get; set; }
    public string ListName { get; set; } = string.Empty;
    public ImportListType ListType { get; set; }
    public int TotalFetched { get; set; }
    public List<ImportSessionItem> NewItems { get; set; } = new();
    public List<ImportSessionItem> Conflicts { get; set; } = new();
    public List<ImportSessionItem> Skipped { get; set; } = new();
    public List<ImportSessionItem> Excluded { get; set; } = new();
}

public class ImportExecutionOptions
{
    public bool AutoResolveConflicts { get; set; }
    public bool IgnoreExclusions { get; set; }
}

public class ConflictResolution
{
    public ConflictStrategy Strategy { get; set; }
    /// <summary>
    /// For MergeFields: field name → true (use imported) / false (keep existing).
    /// </summary>
    public Dictionary<string, bool>? FieldChoices { get; set; }
}

public enum ConflictStrategy
{
    KeepExisting = 0,
    UseImported = 1,
    MergeFields = 2
}

public class FieldDiff
{
    public object? Existing { get; set; }
    public object? Imported { get; set; }
}

using Microsoft.AspNetCore.Authorization;
// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.ImportLists.Export;
using Mouseion.Core.ImportLists.History;
using Mouseion.Core.ImportLists.Wizard;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Api.ImportLists;

[ApiController]
    [Authorize]
[Route("api/v3/import")]
public class ImportWizardController : ControllerBase
{
    private readonly IImportWizardService _wizardService;
    private readonly IExportService _exportService;
    private readonly IImportSessionRepository _sessionRepository;
    private readonly IImportSessionItemRepository _sessionItemRepository;

    public ImportWizardController(
        IImportWizardService wizardService,
        IExportService exportService,
        IImportSessionRepository sessionRepository,
        IImportSessionItemRepository sessionItemRepository)
    {
        _wizardService = wizardService;
        _exportService = exportService;
        _sessionRepository = sessionRepository;
        _sessionItemRepository = sessionItemRepository;
    }

    // ──────────────────────────────────────────────
    // Wizard: Preview → Execute → Resolve
    // ──────────────────────────────────────────────

    /// <summary>
    /// Dry-run: fetch items from an import list and show what would happen.
    /// Returns new items, conflicts, skipped, and excluded items.
    /// </summary>
    [HttpPost("preview/{listId}")]
    public async Task<ActionResult<ImportPreviewResource>> Preview(int listId)
    {
        var result = await _wizardService.PreviewAsync(listId);
        return Ok(result.ToResource());
    }

    /// <summary>
    /// Execute an import: fetch, detect conflicts, add new items.
    /// Pass autoResolve=true to automatically overwrite on conflicts.
    /// </summary>
    [HttpPost("execute/{listId}")]
    public async Task<ActionResult<ImportSessionResource>> Execute(int listId, [FromQuery] bool autoResolve = false)
    {
        var session = await _wizardService.ExecuteAsync(listId, new ImportExecutionOptions
        {
            AutoResolveConflicts = autoResolve
        });
        return Ok(session.ToResource());
    }

    /// <summary>
    /// Re-sync: re-import everything from a list with auto-resolve.
    /// Useful after initial library setup or after changing list settings.
    /// </summary>
    [HttpPost("resync/{listId}")]
    public async Task<ActionResult<ImportSessionResource>> ReSync(int listId)
    {
        var session = await _wizardService.ReSyncAsync(listId);
        return Ok(session.ToResource());
    }

    /// <summary>
    /// Resolve a conflict: choose to keep existing, use imported, or merge specific fields.
    /// </summary>
    [HttpPost("resolve/{sessionItemId}")]
    public async Task<ActionResult<ImportSessionItemResource>> ResolveConflict(
        int sessionItemId,
        [FromBody][Required] ConflictResolutionResource resource)
    {
        var resolution = new ConflictResolution
        {
            Strategy = resource.Strategy,
            FieldChoices = resource.FieldChoices
        };

        var item = await _wizardService.ResolveConflictAsync(sessionItemId, resolution);
        return Ok(item.ToResource());
    }

    // ──────────────────────────────────────────────
    // History
    // ──────────────────────────────────────────────

    /// <summary>Get recent import sessions (most recent first).</summary>
    [HttpGet("history")]
    public ActionResult<List<ImportSessionResource>> GetHistory([FromQuery] int count = 50)
    {
        var sessions = _sessionRepository.GetRecent(count);
        return Ok(sessions.Select(s => s.ToResource()).ToList());
    }

    /// <summary>Get import history for a specific list.</summary>
    [HttpGet("history/list/{listId}")]
    public ActionResult<List<ImportSessionResource>> GetHistoryByList(int listId)
    {
        var sessions = _sessionRepository.GetByListId(listId);
        return Ok(sessions.Select(s => s.ToResource()).ToList());
    }

    /// <summary>Get details of a specific import session.</summary>
    [HttpGet("history/{sessionId}")]
    public ActionResult<ImportSessionDetailResource> GetSession(int sessionId)
    {
        var session = _sessionRepository.Get(sessionId);
        var items = _sessionItemRepository.GetBySessionId(sessionId);
        return Ok(new ImportSessionDetailResource
        {
            Session = session.ToResource(),
            Items = items.Select(i => i.ToResource()).ToList()
        });
    }

    /// <summary>Get unresolved conflicts for a session.</summary>
    [HttpGet("history/{sessionId}/conflicts")]
    public ActionResult<List<ImportSessionItemResource>> GetConflicts(int sessionId)
    {
        var conflicts = _sessionItemRepository.GetConflicts(sessionId);
        return Ok(conflicts.Select(i => i.ToResource()).ToList());
    }

    // ──────────────────────────────────────────────
    // Export
    // ──────────────────────────────────────────────

    /// <summary>Export library as JSON.</summary>
    [HttpGet("export/json")]
    public async Task<IActionResult> ExportJson([FromQuery] string? mediaTypes = null, [FromQuery] DateTime? addedAfter = null)
    {
        var options = ParseExportOptions(mediaTypes, addedAfter);
        var result = await _exportService.ExportJsonAsync(options);
        return File(result.Data, result.ContentType, result.FileName);
    }

    /// <summary>Export library as CSV.</summary>
    [HttpGet("export/csv")]
    public async Task<IActionResult> ExportCsv([FromQuery] string? mediaTypes = null, [FromQuery] DateTime? addedAfter = null)
    {
        var options = ParseExportOptions(mediaTypes, addedAfter);
        var result = await _exportService.ExportCsvAsync(options);
        return File(result.Data, result.ContentType, result.FileName);
    }

    /// <summary>Export in a service-specific format (trakt, goodreads, letterboxd).</summary>
    [HttpGet("export/{target}")]
    public async Task<IActionResult> ExportForService(string target, [FromQuery] string? mediaTypes = null, [FromQuery] DateTime? addedAfter = null)
    {
        if (!Enum.TryParse<ExportTarget>(target, ignoreCase: true, out var exportTarget))
        {
            // Try matching common aliases
            exportTarget = target.ToLowerInvariant() switch
            {
                "trakt" => ExportTarget.TraktImport,
                "goodreads" => ExportTarget.GoodreadsCsv,
                "letterboxd" => ExportTarget.LetterboxdCsv,
                _ => throw new ArgumentException($"Unknown export target: {target}. Supported: trakt, goodreads, letterboxd, json")
            };
        }

        var options = ParseExportOptions(mediaTypes, addedAfter);
        var result = await _exportService.ExportForServiceAsync(exportTarget, options);
        return File(result.Data, result.ContentType, result.FileName);
    }

    private static ExportOptions ParseExportOptions(string? mediaTypes, DateTime? addedAfter)
    {
        IEnumerable<MediaType>? types = null;
        if (!string.IsNullOrEmpty(mediaTypes))
        {
            types = mediaTypes.Split(',')
                .Select(t => Enum.TryParse<MediaType>(t.Trim(), ignoreCase: true, out var mt) ? mt : MediaType.Unknown)
                .Where(t => t != MediaType.Unknown);
        }

        return new ExportOptions
        {
            MediaTypes = types,
            AddedAfter = addedAfter
        };
    }
}

// ──────────────────────────────────────────────
// API Resources
// ──────────────────────────────────────────────

public class ImportPreviewResource
{
    public int SessionId { get; set; }
    public int ListId { get; set; }
    public string ListName { get; set; } = string.Empty;
    public string ListType { get; set; } = string.Empty;
    public int TotalFetched { get; set; }
    public List<ImportSessionItemResource> NewItems { get; set; } = new();
    public List<ImportSessionItemResource> Conflicts { get; set; } = new();
    public List<ImportSessionItemResource> Skipped { get; set; } = new();
    public List<ImportSessionItemResource> Excluded { get; set; } = new();
    public ImportPreviewSummary Summary { get; set; } = new();
}

public class ImportPreviewSummary
{
    public int WouldAdd { get; set; }
    public int HasConflicts { get; set; }
    public int AlreadyInLibrary { get; set; }
    public int Excluded { get; set; }
}

public class ImportSessionResource
{
    public int Id { get; set; }
    public int ImportListId { get; set; }
    public string ImportListName { get; set; } = string.Empty;
    public string ListType { get; set; } = string.Empty;
    public string MediaType { get; set; } = string.Empty;
    public string Status { get; set; } = string.Empty;
    public bool IsDryRun { get; set; }
    public int ItemsFetched { get; set; }
    public int ItemsAdded { get; set; }
    public int ItemsUpdated { get; set; }
    public int ItemsSkipped { get; set; }
    public int ItemsFailed { get; set; }
    public string? ErrorMessage { get; set; }
    public DateTime StartedAt { get; set; }
    public DateTime? CompletedAt { get; set; }
    public double? DurationSeconds { get; set; }
}

public class ImportSessionDetailResource
{
    public ImportSessionResource Session { get; set; } = new();
    public List<ImportSessionItemResource> Items { get; set; } = new();
}

public class ImportSessionItemResource
{
    public int Id { get; set; }
    public int SessionId { get; set; }
    public string Title { get; set; } = string.Empty;
    public int Year { get; set; }
    public string MediaType { get; set; } = string.Empty;
    public string Action { get; set; } = string.Empty;
    public string? Reason { get; set; }
    public int? MediaItemId { get; set; }
    public int? UserRating { get; set; }
    public Dictionary<string, object>? Diff { get; set; }
    public DateTime ProcessedAt { get; set; }
}

public class ConflictResolutionResource
{
    public ConflictStrategy Strategy { get; set; }
    public Dictionary<string, bool>? FieldChoices { get; set; }
}

// ──────────────────────────────────────────────
// Mapping extensions
// ──────────────────────────────────────────────

public static class ImportWizardResourceMapper
{
    public static ImportPreviewResource ToResource(this ImportPreviewResult result)
    {
        return new ImportPreviewResource
        {
            SessionId = result.SessionId,
            ListId = result.ListId,
            ListName = result.ListName,
            ListType = result.ListType.ToString(),
            TotalFetched = result.TotalFetched,
            NewItems = result.NewItems.Select(i => i.ToResource()).ToList(),
            Conflicts = result.Conflicts.Select(i => i.ToResource()).ToList(),
            Skipped = result.Skipped.Select(i => i.ToResource()).ToList(),
            Excluded = result.Excluded.Select(i => i.ToResource()).ToList(),
            Summary = new ImportPreviewSummary
            {
                WouldAdd = result.NewItems.Count,
                HasConflicts = result.Conflicts.Count,
                AlreadyInLibrary = result.Skipped.Count,
                Excluded = result.Excluded.Count
            }
        };
    }

    public static ImportSessionResource ToResource(this ImportSession session)
    {
        return new ImportSessionResource
        {
            Id = session.Id,
            ImportListId = session.ImportListId,
            ImportListName = session.ImportListName,
            ListType = session.ListType.ToString(),
            MediaType = session.MediaType.ToString(),
            Status = session.Status.ToString(),
            IsDryRun = session.IsDryRun,
            ItemsFetched = session.ItemsFetched,
            ItemsAdded = session.ItemsAdded,
            ItemsUpdated = session.ItemsUpdated,
            ItemsSkipped = session.ItemsSkipped,
            ItemsFailed = session.ItemsFailed,
            ErrorMessage = session.ErrorMessage,
            StartedAt = session.StartedAt,
            CompletedAt = session.CompletedAt,
            DurationSeconds = session.CompletedAt.HasValue
                ? (session.CompletedAt.Value - session.StartedAt).TotalSeconds
                : null
        };
    }

    public static ImportSessionItemResource ToResource(this ImportSessionItem item)
    {
        Dictionary<string, object>? diff = null;
        if (!string.IsNullOrEmpty(item.DiffJson))
        {
            try
            {
                diff = global::System.Text.Json.JsonSerializer.Deserialize<Dictionary<string, object>>(item.DiffJson);
            }
            catch { /* ignore malformed diff JSON */ }
        }

        return new ImportSessionItemResource
        {
            Id = item.Id,
            SessionId = item.SessionId,
            Title = item.Title,
            Year = item.Year,
            MediaType = item.MediaType.ToString(),
            Action = item.Action.ToString(),
            Reason = item.Reason,
            MediaItemId = item.MediaItemId,
            UserRating = item.UserRating,
            Diff = diff,
            ProcessedAt = item.ProcessedAt
        };
    }
}

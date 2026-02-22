// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.ImportLists.History;

public class ImportSession : ModelBase
{
    public int ImportListId { get; set; }
    public string ImportListName { get; set; } = string.Empty;
    public ImportListType ListType { get; set; }
    public MediaType MediaType { get; set; }
    public ImportSessionStatus Status { get; set; }
    public bool IsDryRun { get; set; }
    public int ItemsFetched { get; set; }
    public int ItemsAdded { get; set; }
    public int ItemsUpdated { get; set; }
    public int ItemsSkipped { get; set; }
    public int ItemsFailed { get; set; }
    public string? ErrorMessage { get; set; }
    public DateTime StartedAt { get; set; }
    public DateTime? CompletedAt { get; set; }
}

public enum ImportSessionStatus
{
    Pending = 0,
    Running = 1,
    Completed = 2,
    Failed = 3,
    DryRun = 4
}

public class ImportSessionItem : ModelBase
{
    public int SessionId { get; set; }
    public string Title { get; set; } = string.Empty;
    public int Year { get; set; }
    public MediaType MediaType { get; set; }
    public ImportItemAction Action { get; set; }
    public string? Reason { get; set; }
    public string? ExternalIds { get; set; }
    public int? MediaItemId { get; set; }
    public string? DiffJson { get; set; }
    public int? UserRating { get; set; }
    public DateTime ProcessedAt { get; set; }
}

public enum ImportItemAction
{
    Added = 0,
    Updated = 1,
    Skipped = 2,
    Conflict = 3,
    Failed = 4,
    Excluded = 5
}

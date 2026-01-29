// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Bulk;

public class BulkUpdateRequest
{
    public List<BulkUpdateItem> Items { get; set; } = new();
}

public class BulkUpdateItem
{
    public int Id { get; set; }
    public bool? Monitored { get; set; }
    public int? QualityProfileId { get; set; }
    public string? Path { get; set; }
    public string? RootFolderPath { get; set; }
    public List<int>? Tags { get; set; }
}

public class BulkUpdateResult
{
    public int Updated { get; set; }
    public List<int> UpdatedIds { get; set; } = new();
    public List<BulkError>? Errors { get; set; }
}

public class BulkDeleteRequest
{
    public List<int> Ids { get; set; } = new();
    public bool DeleteFiles { get; set; }
}

public class BulkDeleteResult
{
    public int Deleted { get; set; }
    public List<int> DeletedIds { get; set; } = new();
    public List<BulkError>? Errors { get; set; }
}

public class BulkError
{
    public int Id { get; set; }
    public string Message { get; set; } = string.Empty;
}

public class BulkReadRequest
{
    public List<int> Ids { get; set; } = new();
    public bool IsRead { get; set; } = true;
}

public class BulkReadResult
{
    public int Updated { get; set; }
    public List<int> UpdatedIds { get; set; } = new();
}

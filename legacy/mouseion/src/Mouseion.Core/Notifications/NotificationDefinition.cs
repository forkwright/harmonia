// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Notifications;

/// <summary>
/// Database entity for notification configuration
/// </summary>
public class NotificationDefinition : ModelBase
{
    public string Name { get; set; } = string.Empty;

    public string Implementation { get; set; } = string.Empty;

    public string ConfigContract { get; set; } = string.Empty;

    public string? Settings { get; set; }

    public bool Enabled { get; set; }

    public bool OnGrab { get; set; }

    public bool OnDownload { get; set; }

    public bool OnRename { get; set; }

    public bool OnMediaAdded { get; set; }

    public bool OnMediaDeleted { get; set; }

    public bool OnHealthIssue { get; set; }

    public bool OnHealthRestored { get; set; }

    public bool OnApplicationUpdate { get; set; }

    public HashSet<int> Tags { get; set; } = new();
}

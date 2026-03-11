// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Authentication;

/// <summary>
/// Per-user preferences: hidden media types, quality overrides, UI settings.
/// Each user gets one row — created on first login or by admin.
/// </summary>
public class UserPreferences : ModelBase
{
    public int UserId { get; set; }
    public string? HiddenMediaTypes { get; set; } // JSON: [7, 8]
    public int? DefaultQualityProfileId { get; set; }
    public string? QualityProfileOverrides { get; set; } // JSON: {"1": 3, "2": 5} = Movie→profile 3, TV→profile 5
    public string? Language { get; set; }
    public string? Theme { get; set; }
    public bool NotificationsEnabled { get; set; } = true;
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
    public DateTime UpdatedAt { get; set; } = DateTime.UtcNow;
}

/// <summary>
/// Per-user smart list subscription: user opts in to specific smart lists
/// with auto-add and notification preferences.
/// </summary>
public class UserSmartListSubscription : ModelBase
{
    public int UserId { get; set; }
    public int SmartListId { get; set; }
    public bool AutoAdd { get; set; }
    public bool NotifyOnNew { get; set; } = true;
    public DateTime SubscribedAt { get; set; } = DateTime.UtcNow;
}

/// <summary>
/// Resource-level permission: restrict a user to specific media types, root folders,
/// or capabilities (downloads).
/// </summary>
public class UserPermission : ModelBase
{
    public int UserId { get; set; }
    public PermissionType PermissionType { get; set; }
    public string ResourceId { get; set; } = string.Empty;
    public bool Allowed { get; set; } = true;
    public int? GrantedBy { get; set; }
    public DateTime GrantedAt { get; set; } = DateTime.UtcNow;
}

public enum PermissionType
{
    MediaTypeAccess = 0,    // ResourceId = MediaType int
    RootFolderAccess = 1,   // ResourceId = folder path
    DownloadPermission = 2  // ResourceId = "search" | "download" | "import"
}

/// <summary>
/// Scoped API key: tied to a user, limited by scopes, with expiration.
/// </summary>
public class ApiKey : ModelBase
{
    public int UserId { get; set; }
    public string Name { get; set; } = string.Empty;
    public string KeyHash { get; set; } = string.Empty;
    public string KeyPrefix { get; set; } = string.Empty;
    public string? Scopes { get; set; } // JSON: ["read", "progress", "download", "admin"]
    public DateTime? ExpiresAt { get; set; }
    public DateTime? LastUsedAt { get; set; }
    public bool IsActive { get; set; } = true;
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
}

/// <summary>
/// Audit log entry: tracks authentication events, permission changes, admin actions.
/// </summary>
public class AuditLogEntry : ModelBase
{
    public int? UserId { get; set; }
    public string Action { get; set; } = string.Empty;
    public string? ResourceType { get; set; }
    public string? ResourceId { get; set; }
    public string? Details { get; set; } // JSON context
    public string? IpAddress { get; set; }
    public string? UserAgent { get; set; }
    public DateTime Timestamp { get; set; } = DateTime.UtcNow;
}

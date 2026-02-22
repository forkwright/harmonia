// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Webhooks;

/// <summary>
/// Persisted record of a webhook event received from an external media server.
/// Used for deduplication and audit trail.
/// </summary>
public class WebhookEvent : ModelBase
{
    public WebhookSource Source { get; set; }
    public string EventType { get; set; } = string.Empty;
    public string ExternalItemId { get; set; } = string.Empty;
    public string? ExternalUserId { get; set; }
    public int? ResolvedMediaItemId { get; set; }
    public string RawPayload { get; set; } = string.Empty;
    public bool Processed { get; set; }
    public string? Error { get; set; }
    public DateTime ReceivedAt { get; set; } = DateTime.UtcNow;
    public DateTime? ProcessedAt { get; set; }
}

public enum WebhookSource
{
    Jellyfin = 0,
    Emby = 1,
    Plex = 2
}

public enum WebhookEventType
{
    PlaybackStart,
    PlaybackPause,
    PlaybackStop,
    PlaybackProgress,
    MarkWatched,
    MarkUnwatched
}

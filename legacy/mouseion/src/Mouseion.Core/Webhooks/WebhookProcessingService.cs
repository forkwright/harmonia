// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.Progress;

namespace Mouseion.Core.Webhooks;

/// <summary>
/// Processes webhook events from external media servers (Jellyfin, Emby, Plex).
/// Resolves external media IDs to Mouseion IDs and creates/updates progress records.
/// </summary>
public interface IWebhookProcessingService
{
    Task<WebhookResult> ProcessJellyfinAsync(JellyfinWebhookPayload payload, CancellationToken ct = default);
    Task<WebhookResult> ProcessEmbyAsync(EmbyWebhookPayload payload, CancellationToken ct = default);
    Task<WebhookResult> ProcessPlexAsync(PlexWebhookPayload payload, CancellationToken ct = default);
}

public class WebhookProcessingService : IWebhookProcessingService
{
    private readonly WebhookEventRepository _eventRepo;
    private readonly MediaProgressRepository _progressRepo;
    private readonly PlaybackSessionRepository _sessionRepo;
    private readonly IExternalMediaResolver _resolver;
    private readonly ILogger<WebhookProcessingService> _logger;

    private static readonly TimeSpan DeduplicationWindow = TimeSpan.FromSeconds(30);
    private const decimal DefaultCompletionThreshold = 0.90m;

    public WebhookProcessingService(
        WebhookEventRepository eventRepo,
        MediaProgressRepository progressRepo,
        PlaybackSessionRepository sessionRepo,
        IExternalMediaResolver resolver,
        ILogger<WebhookProcessingService> logger)
    {
        _eventRepo = eventRepo;
        _progressRepo = progressRepo;
        _sessionRepo = sessionRepo;
        _resolver = resolver;
        _logger = logger;
    }

    public async Task<WebhookResult> ProcessJellyfinAsync(JellyfinWebhookPayload payload, CancellationToken ct = default)
    {
        var eventType = MapJellyfinEventType(payload.NotificationType);
        if (eventType == null)
        {
            return WebhookResult.Ignored($"Unknown Jellyfin event type: {payload.NotificationType}");
        }

        var externalInfo = new ExternalMediaInfo
        {
            TmdbId = payload.Provider_tmdb,
            ImdbId = payload.Provider_imdb,
            TvdbId = ParseIntOrNull(payload.Provider_tvdb),
            Title = payload.Name,
            Year = payload.Year,
            SeasonNumber = payload.SeasonNumber,
            EpisodeNumber = payload.EpisodeNumber
        };

        return await ProcessEventAsync(
            WebhookSource.Jellyfin,
            eventType.Value,
            payload.ItemId ?? string.Empty,
            payload.UserId,
            externalInfo,
            payload.PlaybackPositionTicks.HasValue ? payload.PlaybackPositionTicks.Value / TimeSpan.TicksPerMillisecond : null,
            payload.RunTimeTicks.HasValue ? payload.RunTimeTicks.Value / TimeSpan.TicksPerMillisecond : null,
            payload.DeviceName,
            System.Text.Json.JsonSerializer.Serialize(payload),
            ct).ConfigureAwait(false);
    }

    public async Task<WebhookResult> ProcessEmbyAsync(EmbyWebhookPayload payload, CancellationToken ct = default)
    {
        var eventType = MapEmbyEventType(payload.Event);
        if (eventType == null)
        {
            return WebhookResult.Ignored($"Unknown Emby event type: {payload.Event}");
        }

        var externalInfo = new ExternalMediaInfo
        {
            TmdbId = payload.Item?.ProviderIds?.GetValueOrDefault("Tmdb"),
            ImdbId = payload.Item?.ProviderIds?.GetValueOrDefault("Imdb"),
            TvdbId = ParseIntOrNull(payload.Item?.ProviderIds?.GetValueOrDefault("Tvdb")),
            Title = payload.Item?.Name,
            Year = payload.Item?.ProductionYear,
            SeasonNumber = payload.Item?.ParentIndexNumber,
            EpisodeNumber = payload.Item?.IndexNumber
        };

        return await ProcessEventAsync(
            WebhookSource.Emby,
            eventType.Value,
            payload.Item?.Id ?? string.Empty,
            payload.User?.Id,
            externalInfo,
            payload.PlaybackInfo?.PositionTicks.HasValue == true ? payload.PlaybackInfo.PositionTicks.Value / TimeSpan.TicksPerMillisecond : null,
            payload.Item?.RunTimeTicks.HasValue == true ? payload.Item.RunTimeTicks.Value / TimeSpan.TicksPerMillisecond : null,
            payload.DeviceName,
            System.Text.Json.JsonSerializer.Serialize(payload),
            ct).ConfigureAwait(false);
    }

    public async Task<WebhookResult> ProcessPlexAsync(PlexWebhookPayload payload, CancellationToken ct = default)
    {
        var eventType = MapPlexEventType(payload.Event);
        if (eventType == null)
        {
            return WebhookResult.Ignored($"Unknown Plex event type: {payload.Event}");
        }

        var externalInfo = new ExternalMediaInfo
        {
            Title = payload.Metadata?.Title,
            Year = payload.Metadata?.Year
        };

        // Plex provides GUIDs like "com.plexapp.agents.imdb://tt1234567"
        if (payload.Metadata?.ExternalGuids != null)
        {
            foreach (var guid in payload.Metadata.ExternalGuids)
            {
                if (guid.Id?.StartsWith("imdb://") == true)
                    externalInfo.ImdbId = guid.Id["imdb://".Length..];
                else if (guid.Id?.StartsWith("tmdb://") == true)
                    externalInfo.TmdbId = guid.Id["tmdb://".Length..];
                else if (guid.Id?.StartsWith("tvdb://") == true && int.TryParse(guid.Id["tvdb://".Length..], out var tvdbId))
                    externalInfo.TvdbId = tvdbId;
            }
        }

        return await ProcessEventAsync(
            WebhookSource.Plex,
            eventType.Value,
            payload.Metadata?.RatingKey ?? string.Empty,
            payload.Account?.Id?.ToString(),
            externalInfo,
            payload.Metadata?.ViewOffset,
            payload.Metadata?.Duration,
            payload.Player?.Title,
            System.Text.Json.JsonSerializer.Serialize(payload),
            ct).ConfigureAwait(false);
    }

    private async Task<WebhookResult> ProcessEventAsync(
        WebhookSource source,
        WebhookEventType eventType,
        string externalItemId,
        string? externalUserId,
        ExternalMediaInfo mediaInfo,
        long? positionMs,
        long? durationMs,
        string? deviceName,
        string rawPayload,
        CancellationToken ct)
    {
        // Deduplication: skip if we received the same event recently
        var duplicate = await _eventRepo.FindRecentDuplicate(
            source, externalItemId, eventType.ToString(), DeduplicationWindow, ct).ConfigureAwait(false);

        if (duplicate != null)
        {
            _logger.LogDebug("Skipping duplicate {Source} webhook: {EventType} for {ItemId}",
                source, eventType, externalItemId);
            return WebhookResult.Deduplicated();
        }

        // Persist the event
        var webhookEvent = new WebhookEvent
        {
            Source = source,
            EventType = eventType.ToString(),
            ExternalItemId = externalItemId,
            ExternalUserId = externalUserId,
            RawPayload = rawPayload,
            ReceivedAt = DateTime.UtcNow
        };

        // Resolve external media to Mouseion ID
        var mediaItemId = await _resolver.ResolveAsync(mediaInfo, ct).ConfigureAwait(false);

        if (mediaItemId == null)
        {
            webhookEvent.Error = "Could not resolve external media to Mouseion item";
            webhookEvent.Processed = false;
            _eventRepo.Insert(webhookEvent);

            _logger.LogInformation("{Source} webhook received but media not resolved: {Title} ({EventType})",
                source, mediaInfo.Title, eventType);
            return WebhookResult.UnresolvedMedia(mediaInfo.Title);
        }

        webhookEvent.ResolvedMediaItemId = mediaItemId.Value;

        // Update progress/session based on event type
        try
        {
            await UpdateProgressAsync(eventType, mediaItemId.Value, positionMs, durationMs, deviceName, ct).ConfigureAwait(false);
            webhookEvent.Processed = true;
            webhookEvent.ProcessedAt = DateTime.UtcNow;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error processing {Source} webhook for MediaItem {Id}", source, mediaItemId.Value);
            webhookEvent.Error = ex.Message;
            webhookEvent.Processed = false;
        }

        _eventRepo.Insert(webhookEvent);

        return webhookEvent.Processed
            ? WebhookResult.Success(mediaItemId.Value)
            : WebhookResult.Failed(webhookEvent.Error!);
    }

    private async Task UpdateProgressAsync(
        WebhookEventType eventType,
        int mediaItemId,
        long? positionMs,
        long? durationMs,
        string? deviceName,
        CancellationToken ct)
    {
        var progress = await _progressRepo.FindByMediaItemAsync(mediaItemId, ct).ConfigureAwait(false);

        switch (eventType)
        {
            case WebhookEventType.PlaybackStart:
                // Create/update progress, start a new session
                progress = EnsureProgress(progress, mediaItemId, positionMs, durationMs);
                _progressRepo.Upsert(progress);

                var session = new PlaybackSession
                {
                    MediaItemId = mediaItemId,
                    DeviceName = deviceName ?? "External",
                    DeviceType = "MediaServer",
                    StartedAt = DateTime.UtcNow,
                    StartPositionMs = positionMs ?? 0,
                    IsActive = true
                };
                _sessionRepo.Insert(session);
                break;

            case WebhookEventType.PlaybackProgress:
            case WebhookEventType.PlaybackPause:
                progress = EnsureProgress(progress, mediaItemId, positionMs, durationMs);
                _progressRepo.Upsert(progress);
                break;

            case WebhookEventType.PlaybackStop:
                progress = EnsureProgress(progress, mediaItemId, positionMs, durationMs);

                // Auto-complete if past threshold
                if (progress.PercentComplete >= DefaultCompletionThreshold * 100)
                {
                    progress.IsComplete = true;
                }

                _progressRepo.Upsert(progress);

                // Close active sessions for this item
                var activeSessions = await _sessionRepo.GetActiveByMediaItemAsync(mediaItemId, ct).ConfigureAwait(false);
                foreach (var active in activeSessions)
                {
                    active.IsActive = false;
                    active.EndedAt = DateTime.UtcNow;
                    active.EndPositionMs = positionMs;
                    active.DurationMs = (long)(DateTime.UtcNow - active.StartedAt).TotalMilliseconds;
                    _sessionRepo.Update(active);
                }
                break;

            case WebhookEventType.MarkWatched:
                progress = EnsureProgress(progress, mediaItemId, durationMs, durationMs);
                progress.IsComplete = true;
                progress.PercentComplete = 100;
                _progressRepo.Upsert(progress);
                break;

            case WebhookEventType.MarkUnwatched:
                if (progress != null)
                {
                    progress.IsComplete = false;
                    progress.PositionMs = 0;
                    progress.PercentComplete = 0;
                    _progressRepo.Upsert(progress);
                }
                break;
        }
    }

    private static MediaProgress EnsureProgress(MediaProgress? existing, int mediaItemId, long? positionMs, long? durationMs)
    {
        var progress = existing ?? new MediaProgress
        {
            MediaItemId = mediaItemId,
            CreatedAt = DateTime.UtcNow
        };

        if (positionMs.HasValue)
            progress.PositionMs = positionMs.Value;

        if (durationMs.HasValue && durationMs.Value > 0)
        {
            progress.TotalDurationMs = durationMs.Value;
            progress.PercentComplete = Math.Min(100, (decimal)progress.PositionMs / durationMs.Value * 100);
        }

        progress.LastPlayedAt = DateTime.UtcNow;
        progress.UpdatedAt = DateTime.UtcNow;

        return progress;
    }

    private static WebhookEventType? MapJellyfinEventType(string? notificationType)
    {
        return notificationType?.ToLowerInvariant() switch
        {
            "playbackstart" => WebhookEventType.PlaybackStart,
            "playbackprogress" => WebhookEventType.PlaybackProgress,
            "playbackstop" => WebhookEventType.PlaybackStop,
            "usermarkplayeditem" or "markplayed" => WebhookEventType.MarkWatched,
            "usermarkunplayeditem" or "markunplayed" => WebhookEventType.MarkUnwatched,
            _ => null
        };
    }

    private static WebhookEventType? MapEmbyEventType(string? eventName)
    {
        return eventName?.ToLowerInvariant() switch
        {
            "playback.start" => WebhookEventType.PlaybackStart,
            "playback.progress" => WebhookEventType.PlaybackProgress,
            "playback.stop" => WebhookEventType.PlaybackStop,
            "item.markedplayed" => WebhookEventType.MarkWatched,
            "item.markedunplayed" => WebhookEventType.MarkUnwatched,
            _ => null
        };
    }

    private static WebhookEventType? MapPlexEventType(string? eventName)
    {
        return eventName?.ToLowerInvariant() switch
        {
            "media.play" => WebhookEventType.PlaybackStart,
            "media.resume" => WebhookEventType.PlaybackStart,
            "media.pause" => WebhookEventType.PlaybackPause,
            "media.stop" => WebhookEventType.PlaybackStop,
            "media.scrobble" => WebhookEventType.MarkWatched,
            _ => null
        };
    }

    private static int? ParseIntOrNull(string? value)
    {
        return int.TryParse(value, out var result) ? result : null;
    }
}

public class WebhookResult
{
    public bool IsSuccess { get; init; }
    public string Status { get; init; } = string.Empty;
    public int? MediaItemId { get; init; }
    public string? Message { get; init; }

    public static WebhookResult Success(int mediaItemId) => new() { IsSuccess = true, Status = "processed", MediaItemId = mediaItemId };
    public static WebhookResult Deduplicated() => new() { IsSuccess = true, Status = "deduplicated" };
    public static WebhookResult Ignored(string reason) => new() { IsSuccess = true, Status = "ignored", Message = reason };
    public static WebhookResult UnresolvedMedia(string? title) => new() { IsSuccess = false, Status = "unresolved", Message = $"Could not match '{title}' to a Mouseion library item" };
    public static WebhookResult Failed(string error) => new() { IsSuccess = false, Status = "failed", Message = error };
}

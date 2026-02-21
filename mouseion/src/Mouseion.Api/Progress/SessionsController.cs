// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using System.Security.Claims;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Progress;
using Mouseion.Core.MediaItems;

namespace Mouseion.Api.Progress;

[ApiController]
[Route("api/v3/sessions")]
[Authorize]
public class SessionsController : ControllerBase
{
    private readonly IPlaybackSessionRepository _sessionRepository;
    private readonly IMediaItemRepository _mediaItemRepository;
    private readonly IMediaProgressRepository _progressRepository;

    public SessionsController(
        IPlaybackSessionRepository sessionRepository,
        IMediaItemRepository mediaItemRepository,
        IMediaProgressRepository progressRepository)
    {
        _sessionRepository = sessionRepository;
        _mediaItemRepository = mediaItemRepository;
        _progressRepository = progressRepository;
    }

    [HttpGet]
    public async Task<ActionResult<List<PlaybackSessionResource>>> GetSessions(
        [FromQuery] bool activeOnly = false,
        [FromQuery] int limit = 100,
        CancellationToken ct = default)
    {
        var userId = GetCurrentUserId();
        var sessions = activeOnly
            ? await _sessionRepository.GetActiveSessionsAsync(userId.ToString(), ct).ConfigureAwait(false)
            : await _sessionRepository.GetRecentSessionsAsync(userId.ToString(), limit, ct).ConfigureAwait(false);

        return Ok(sessions.Select(ToResource).ToList());
    }

    [HttpGet("{sessionId}")]
    public async Task<ActionResult<PlaybackSessionResource>> GetSession(
        string sessionId,
        CancellationToken ct = default)
    {
        var session = await _sessionRepository.GetBySessionIdAsync(sessionId, ct).ConfigureAwait(false);
        if (session == null)
        {
            return NotFound(new { error = $"Session {sessionId} not found" });
        }

        return Ok(ToResource(session));
    }

    [HttpGet("media/{mediaItemId:int}")]
    public async Task<ActionResult<List<PlaybackSessionResource>>> GetSessionsByMediaItem(
        int mediaItemId,
        CancellationToken ct = default)
    {
        var userId = GetCurrentUserId();
        var sessions = await _sessionRepository.GetByMediaItemIdAsync(mediaItemId, userId.ToString(), ct).ConfigureAwait(false);
        return Ok(sessions.Select(ToResource).ToList());
    }

    [HttpPost]
    public async Task<ActionResult<PlaybackSessionResource>> StartSession(
        [FromBody][Required] StartSessionRequest request,
        CancellationToken ct = default)
    {
        var mediaItem = await _mediaItemRepository.FindByIdAsync(request.MediaItemId, ct).ConfigureAwait(false);
        if (mediaItem == null)
        {
            return NotFound(new { error = $"Media item {request.MediaItemId} not found" });
        }

        var userId = GetCurrentUserId();

        // End any existing active sessions for this user+device (one active session per device)
        var activeSessions = await _sessionRepository.GetActiveSessionsAsync(userId.ToString(), ct).ConfigureAwait(false);
        foreach (var active in activeSessions.Where(s => s.DeviceName == (request.DeviceName ?? "Unknown Device")))
        {
            await _sessionRepository.EndSessionAsync(active.SessionId, active.StartPositionMs, ct).ConfigureAwait(false);
        }

        var session = new PlaybackSession
        {
            SessionId = Guid.NewGuid().ToString(),
            MediaItemId = request.MediaItemId,
            UserId = userId.ToString(),
            UserIdInt = userId,
            DeviceName = request.DeviceName ?? "Unknown Device",
            DeviceType = request.DeviceType ?? "Unknown",
            StartedAt = DateTime.UtcNow,
            StartPositionMs = request.StartPositionMs,
            IsActive = true
        };

        session = await _sessionRepository.InsertAsync(session, ct).ConfigureAwait(false);

        return CreatedAtAction(nameof(GetSession), new { sessionId = session.SessionId }, ToResource(session));
    }

    [HttpPut("{sessionId}")]
    public async Task<ActionResult<PlaybackSessionResource>> UpdateSession(
        string sessionId,
        [FromBody][Required] UpdateSessionRequest request,
        CancellationToken ct = default)
    {
        var session = await _sessionRepository.GetBySessionIdAsync(sessionId, ct).ConfigureAwait(false);
        if (session == null)
        {
            return NotFound(new { error = $"Session {sessionId} not found" });
        }

        if (request.EndSession)
        {
            var endPosition = request.EndPositionMs ?? 0;
            await _sessionRepository.EndSessionAsync(sessionId, endPosition, ct).ConfigureAwait(false);

            // Also update progress when ending a session
            var progress = new MediaProgress
            {
                MediaItemId = session.MediaItemId,
                UserId = session.UserId,
                UserIdInt = session.UserIdInt,
                PositionMs = endPosition,
                TotalDurationMs = request.TotalDurationMs ?? 0,
                PercentComplete = request.TotalDurationMs > 0
                    ? Math.Round((decimal)endPosition / request.TotalDurationMs.Value * 100, 2)
                    : 0,
                LastPlayedAt = DateTime.UtcNow,
                IsComplete = request.MarkComplete ?? (request.TotalDurationMs > 0 && endPosition >= (long)(request.TotalDurationMs.Value * 0.9))
            };
            await _progressRepository.UpsertAsync(progress, ct).ConfigureAwait(false);

            session = await _sessionRepository.GetBySessionIdAsync(sessionId, ct).ConfigureAwait(false);
        }

        return Ok(ToResource(session!));
    }

    [HttpDelete("{sessionId}")]
    public async Task<ActionResult> DeleteSession(string sessionId, CancellationToken ct = default)
    {
        var session = await _sessionRepository.GetBySessionIdAsync(sessionId, ct).ConfigureAwait(false);
        if (session == null)
        {
            return NotFound(new { error = $"Session {sessionId} not found" });
        }

        await _sessionRepository.DeleteAsync(session.Id, ct).ConfigureAwait(false);
        return NoContent();
    }

    private int GetCurrentUserId()
    {
        var userIdClaim = User.FindFirst("userId")?.Value
            ?? User.FindFirst(ClaimTypes.NameIdentifier)?.Value;

        return int.TryParse(userIdClaim, out var id) ? id : 1;
    }

    private static PlaybackSessionResource ToResource(PlaybackSession session)
    {
        return new PlaybackSessionResource
        {
            Id = session.Id,
            SessionId = session.SessionId,
            MediaItemId = session.MediaItemId,
            UserId = session.UserId,
            DeviceName = session.DeviceName,
            DeviceType = session.DeviceType,
            StartedAt = session.StartedAt,
            EndedAt = session.EndedAt,
            StartPositionMs = session.StartPositionMs,
            EndPositionMs = session.EndPositionMs,
            DurationMs = session.DurationMs,
            IsActive = session.IsActive
        };
    }
}

public class PlaybackSessionResource
{
    public int Id { get; set; }
    public string SessionId { get; set; } = string.Empty;
    public int MediaItemId { get; set; }
    public string UserId { get; set; } = string.Empty;
    public string DeviceName { get; set; } = string.Empty;
    public string DeviceType { get; set; } = string.Empty;
    public DateTime StartedAt { get; set; }
    public DateTime? EndedAt { get; set; }
    public long StartPositionMs { get; set; }
    public long? EndPositionMs { get; set; }
    public long DurationMs { get; set; }
    public bool IsActive { get; set; }
}

public class StartSessionRequest
{
    public int MediaItemId { get; set; }
    public string? UserId { get; set; }
    public string? DeviceName { get; set; }
    public string? DeviceType { get; set; }
    public long StartPositionMs { get; set; }
}

public class UpdateSessionRequest
{
    public bool EndSession { get; set; }
    public long? EndPositionMs { get; set; }
    public long? TotalDurationMs { get; set; }
    public bool? MarkComplete { get; set; }
}

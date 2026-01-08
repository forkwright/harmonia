// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
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

    public SessionsController(
        IPlaybackSessionRepository sessionRepository,
        IMediaItemRepository mediaItemRepository)
    {
        _sessionRepository = sessionRepository;
        _mediaItemRepository = mediaItemRepository;
    }

    [HttpGet]
    public async Task<ActionResult<List<PlaybackSessionResource>>> GetSessions(
        [FromQuery] string userId = "default",
        [FromQuery] bool activeOnly = false,
        [FromQuery] int limit = 100,
        CancellationToken ct = default)
    {
        var sessions = activeOnly
            ? await _sessionRepository.GetActiveSessionsAsync(userId, ct).ConfigureAwait(false)
            : await _sessionRepository.GetRecentSessionsAsync(userId, limit, ct).ConfigureAwait(false);

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
        [FromQuery] string userId = "default",
        CancellationToken ct = default)
    {
        var sessions = await _sessionRepository.GetByMediaItemIdAsync(mediaItemId, userId, ct).ConfigureAwait(false);
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

        var session = new PlaybackSession
        {
            SessionId = Guid.NewGuid().ToString(),
            MediaItemId = request.MediaItemId,
            UserId = request.UserId ?? "default",
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
            await _sessionRepository.EndSessionAsync(sessionId, request.EndPositionMs ?? 0, ct).ConfigureAwait(false);
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
}

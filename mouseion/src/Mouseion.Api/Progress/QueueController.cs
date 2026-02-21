// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using System.Security.Claims;
using System.Text.Json;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Progress;

namespace Mouseion.Api.Progress;

/// <summary>
/// Cross-device queue sync. Each user+device pair has a persistent queue.
/// Clients push queue state on change, pull on connect.
/// </summary>
[ApiController]
[Route("api/v3/queue")]
[Authorize]
public class QueueController : ControllerBase
{
    private readonly IPlaybackQueueRepository _queueRepository;

    public QueueController(IPlaybackQueueRepository queueRepository)
    {
        _queueRepository = queueRepository;
    }

    /// <summary>Get queue state for a specific device (or all devices)</summary>
    [HttpGet]
    public async Task<ActionResult<List<PlaybackQueueResource>>> GetQueues(
        [FromQuery] string? deviceName = null,
        CancellationToken ct = default)
    {
        var userId = GetCurrentUserId();

        if (deviceName != null)
        {
            var queue = await _queueRepository.GetByUserAndDeviceAsync(userId, deviceName, ct).ConfigureAwait(false);
            if (queue == null)
            {
                return Ok(new List<PlaybackQueueResource>());
            }
            return Ok(new List<PlaybackQueueResource> { ToResource(queue) });
        }

        var queues = await _queueRepository.GetByUserAsync(userId, ct).ConfigureAwait(false);
        return Ok(queues.Select(ToResource).ToList());
    }

    /// <summary>Save/update queue state for a device</summary>
    [HttpPut]
    public async Task<ActionResult<PlaybackQueueResource>> SaveQueue(
        [FromBody][Required] SaveQueueRequest request,
        CancellationToken ct = default)
    {
        var userId = GetCurrentUserId();

        var queue = new PlaybackQueue
        {
            UserId = userId,
            DeviceName = request.DeviceName,
            QueueData = JsonSerializer.Serialize(request.Items),
            CurrentIndex = request.CurrentIndex,
            ShuffleEnabled = request.ShuffleEnabled,
            RepeatMode = request.RepeatMode ?? "none"
        };

        await _queueRepository.UpsertAsync(queue, ct).ConfigureAwait(false);

        return Ok(ToResource(queue));
    }

    /// <summary>Transfer playback from one device to another</summary>
    [HttpPost("transfer")]
    public async Task<ActionResult<PlaybackTransferResult>> TransferPlayback(
        [FromBody][Required] PlaybackTransferRequest request,
        CancellationToken ct = default)
    {
        var userId = GetCurrentUserId();

        // Get source device queue
        var sourceQueue = await _queueRepository.GetByUserAndDeviceAsync(userId, request.FromDevice, ct).ConfigureAwait(false);
        if (sourceQueue == null)
        {
            return NotFound(new { error = $"No queue found for device '{request.FromDevice}'" });
        }

        // Copy to target device with optional position override
        var targetQueue = new PlaybackQueue
        {
            UserId = userId,
            DeviceName = request.ToDevice,
            QueueData = sourceQueue.QueueData,
            CurrentIndex = request.CurrentIndex ?? sourceQueue.CurrentIndex,
            ShuffleEnabled = sourceQueue.ShuffleEnabled,
            RepeatMode = sourceQueue.RepeatMode
        };

        await _queueRepository.UpsertAsync(targetQueue, ct).ConfigureAwait(false);

        return Ok(new PlaybackTransferResult
        {
            FromDevice = request.FromDevice,
            ToDevice = request.ToDevice,
            ItemsTransferred = JsonSerializer.Deserialize<List<QueueItem>>(sourceQueue.QueueData)?.Count ?? 0,
            CurrentIndex = targetQueue.CurrentIndex,
            PositionMs = request.PositionMs
        });
    }

    /// <summary>Delete queue for a device</summary>
    [HttpDelete]
    public async Task<ActionResult> DeleteQueue(
        [FromQuery][Required] string deviceName,
        CancellationToken ct = default)
    {
        var userId = GetCurrentUserId();
        var queue = await _queueRepository.GetByUserAndDeviceAsync(userId, deviceName, ct).ConfigureAwait(false);
        if (queue == null)
        {
            return NotFound(new { error = $"No queue found for device '{deviceName}'" });
        }

        await _queueRepository.DeleteAsync(queue.Id, ct).ConfigureAwait(false);
        return NoContent();
    }

    private int GetCurrentUserId()
    {
        var userIdClaim = User.FindFirst("userId")?.Value
            ?? User.FindFirst(ClaimTypes.NameIdentifier)?.Value;
        return int.TryParse(userIdClaim, out var id) ? id : 1;
    }

    private static PlaybackQueueResource ToResource(PlaybackQueue queue)
    {
        return new PlaybackQueueResource
        {
            Id = queue.Id,
            DeviceName = queue.DeviceName,
            Items = JsonSerializer.Deserialize<List<QueueItem>>(queue.QueueData) ?? new(),
            CurrentIndex = queue.CurrentIndex,
            ShuffleEnabled = queue.ShuffleEnabled,
            RepeatMode = queue.RepeatMode,
            UpdatedAt = queue.UpdatedAt
        };
    }
}

public class PlaybackQueueResource
{
    public int Id { get; set; }
    public string DeviceName { get; set; } = string.Empty;
    public List<QueueItem> Items { get; set; } = new();
    public int CurrentIndex { get; set; }
    public bool ShuffleEnabled { get; set; }
    public string RepeatMode { get; set; } = "none";
    public DateTime UpdatedAt { get; set; }
}

public class SaveQueueRequest
{
    public string DeviceName { get; set; } = string.Empty;
    public List<QueueItem> Items { get; set; } = new();
    public int CurrentIndex { get; set; }
    public bool ShuffleEnabled { get; set; }
    public string? RepeatMode { get; set; }
}

public class PlaybackTransferRequest
{
    public string FromDevice { get; set; } = string.Empty;
    public string ToDevice { get; set; } = string.Empty;
    public int? CurrentIndex { get; set; }
    public long? PositionMs { get; set; }
}

public class PlaybackTransferResult
{
    public string FromDevice { get; set; } = string.Empty;
    public string ToDevice { get; set; } = string.Empty;
    public int ItemsTransferred { get; set; }
    public int CurrentIndex { get; set; }
    public long? PositionMs { get; set; }
}

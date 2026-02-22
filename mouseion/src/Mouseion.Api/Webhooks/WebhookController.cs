// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Webhooks;

namespace Mouseion.Api.Webhooks;

/// <summary>
/// Receives playback webhooks from external media servers (Jellyfin, Emby, Plex).
/// Configure your media server to send webhooks to these endpoints.
///
/// Jellyfin: Install jellyfin-plugin-webhook, add Generic Destination pointing to /api/v3/webhooks/jellyfin
/// Emby: Server Settings → Webhooks → add /api/v3/webhooks/emby
/// Plex: Settings → Webhooks → add /api/v3/webhooks/plex (or use Tautulli)
/// </summary>
[ApiController]
[Route("api/v3/webhooks")]
public class WebhookController : ControllerBase
{
    private readonly IWebhookProcessingService _webhookService;

    public WebhookController(IWebhookProcessingService webhookService)
    {
        _webhookService = webhookService;
    }

    /// <summary>
    /// Receive Jellyfin playback webhook events.
    /// </summary>
    [HttpPost("jellyfin")]
    [ProducesResponseType(typeof(WebhookResult), 200)]
    [ProducesResponseType(typeof(WebhookResult), 400)]
    public async Task<IActionResult> Jellyfin([FromBody] JellyfinWebhookPayload payload, CancellationToken ct)
    {
        if (payload == null)
            return BadRequest(WebhookResult.Failed("Empty payload"));

        var result = await _webhookService.ProcessJellyfinAsync(payload, ct);
        return Ok(result);
    }

    /// <summary>
    /// Receive Emby playback webhook events.
    /// </summary>
    [HttpPost("emby")]
    [ProducesResponseType(typeof(WebhookResult), 200)]
    [ProducesResponseType(typeof(WebhookResult), 400)]
    public async Task<IActionResult> Emby([FromBody] EmbyWebhookPayload payload, CancellationToken ct)
    {
        if (payload == null)
            return BadRequest(WebhookResult.Failed("Empty payload"));

        var result = await _webhookService.ProcessEmbyAsync(payload, ct);
        return Ok(result);
    }

    /// <summary>
    /// Receive Plex playback webhook events.
    /// Plex sends webhooks as multipart/form-data with a JSON "payload" field.
    /// </summary>
    [HttpPost("plex")]
    [ProducesResponseType(typeof(WebhookResult), 200)]
    [ProducesResponseType(typeof(WebhookResult), 400)]
    public async Task<IActionResult> Plex(CancellationToken ct)
    {
        PlexWebhookPayload? payload;

        // Plex sends webhooks as multipart/form-data with JSON in a "payload" field
        if (Request.HasFormContentType)
        {
            var form = await Request.ReadFormAsync(ct);
            var payloadJson = form["payload"].ToString();
            if (string.IsNullOrWhiteSpace(payloadJson))
                return BadRequest(WebhookResult.Failed("Missing 'payload' form field"));

            payload = JsonSerializer.Deserialize<PlexWebhookPayload>(payloadJson);
        }
        else
        {
            // Tautulli and some proxies send as application/json
            payload = await JsonSerializer.DeserializeAsync<PlexWebhookPayload>(
                Request.Body, cancellationToken: ct);
        }

        if (payload == null)
            return BadRequest(WebhookResult.Failed("Invalid payload"));

        var result = await _webhookService.ProcessPlexAsync(payload, ct);
        return Ok(result);
    }
}

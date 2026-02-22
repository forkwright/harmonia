// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using System.Security.Claims;
using System.Text.Json;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Authentication;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Api.Security;

/// <summary>
/// Per-user preferences: hidden media types, quality overrides, smart list subscriptions.
/// Each user manages their own preferences; admins can manage any user's.
/// </summary>
[ApiController]
[Authorize]
[Route("api/v3/users")]
public class UserPreferencesController : ControllerBase
{
    private readonly IUserPreferencesRepository _preferencesRepository;
    private readonly IUserSmartListSubscriptionRepository _subscriptionRepository;
    private readonly Mouseion.Core.Authentication.IAuthorizationService _authService;

    public UserPreferencesController(
        IUserPreferencesRepository preferencesRepository,
        IUserSmartListSubscriptionRepository subscriptionRepository,
        Mouseion.Core.Authentication.IAuthorizationService authService)
    {
        _preferencesRepository = preferencesRepository;
        _subscriptionRepository = subscriptionRepository;
        _authService = authService;
    }

    // ──────────────────────────────────────────────
    // Preferences
    // ──────────────────────────────────────────────

    /// <summary>Get current user's preferences.</summary>
    [HttpGet("me/preferences")]
    public async Task<ActionResult<UserPreferencesResource>> GetMyPreferences(CancellationToken ct)
    {
        var userId = GetCurrentUserId();
        var prefs = await _preferencesRepository.GetByUserIdAsync(userId, ct);
        return Ok(ToResource(prefs ?? CreateDefaults(userId)));
    }

    /// <summary>Update current user's preferences.</summary>
    [HttpPut("me/preferences")]
    public async Task<ActionResult<UserPreferencesResource>> UpdateMyPreferences(
        [FromBody][Required] UpdatePreferencesRequest request, CancellationToken ct)
    {
        var userId = GetCurrentUserId();
        var prefs = await _preferencesRepository.GetByUserIdAsync(userId, ct) ?? CreateDefaults(userId);

        if (request.HiddenMediaTypes != null)
            prefs.HiddenMediaTypes = JsonSerializer.Serialize(request.HiddenMediaTypes.Select(t => (int)t));
        if (request.DefaultQualityProfileId.HasValue)
            prefs.DefaultQualityProfileId = request.DefaultQualityProfileId;
        if (request.QualityProfileOverrides != null)
            prefs.QualityProfileOverrides = JsonSerializer.Serialize(request.QualityProfileOverrides);
        if (request.Language != null)
            prefs.Language = request.Language;
        if (request.Theme != null)
            prefs.Theme = request.Theme;
        if (request.NotificationsEnabled.HasValue)
            prefs.NotificationsEnabled = request.NotificationsEnabled.Value;

        await _preferencesRepository.UpsertAsync(prefs, ct);
        return Ok(ToResource(prefs));
    }

    /// <summary>Get a specific user's preferences (admin only).</summary>
    [HttpGet("{userId:int}/preferences")]
    public async Task<ActionResult<UserPreferencesResource>> GetUserPreferences(int userId, CancellationToken ct)
    {
        if (!await IsAdminOrSelf(userId, ct)) return Forbid();

        var prefs = await _preferencesRepository.GetByUserIdAsync(userId, ct);
        return Ok(ToResource(prefs ?? CreateDefaults(userId)));
    }

    // ──────────────────────────────────────────────
    // Smart List Subscriptions
    // ──────────────────────────────────────────────

    /// <summary>Get current user's smart list subscriptions.</summary>
    [HttpGet("me/smartlists")]
    public async Task<ActionResult<List<SmartListSubscriptionResource>>> GetMySubscriptions(CancellationToken ct)
    {
        var userId = GetCurrentUserId();
        var subs = await _subscriptionRepository.GetByUserIdAsync(userId, ct);
        return Ok(subs.Select(ToResource).ToList());
    }

    /// <summary>Subscribe to a smart list.</summary>
    [HttpPost("me/smartlists/{smartListId:int}")]
    public async Task<ActionResult<SmartListSubscriptionResource>> Subscribe(
        int smartListId, [FromBody] SubscribeRequest? request = null, CancellationToken ct = default)
    {
        var userId = GetCurrentUserId();
        var existing = await _subscriptionRepository.GetSubscriptionAsync(userId, smartListId, ct);
        if (existing != null) return Ok(ToResource(existing));

        var sub = new UserSmartListSubscription
        {
            UserId = userId,
            SmartListId = smartListId,
            AutoAdd = request?.AutoAdd ?? false,
            NotifyOnNew = request?.NotifyOnNew ?? true,
            SubscribedAt = DateTime.UtcNow
        };

        var inserted = _subscriptionRepository.Insert(sub);
        return CreatedAtAction(nameof(GetMySubscriptions), null, ToResource(inserted));
    }

    /// <summary>Unsubscribe from a smart list.</summary>
    [HttpDelete("me/smartlists/{smartListId:int}")]
    public async Task<ActionResult> Unsubscribe(int smartListId, CancellationToken ct)
    {
        var userId = GetCurrentUserId();
        await _subscriptionRepository.DeleteSubscriptionAsync(userId, smartListId, ct);
        return NoContent();
    }

    // ──────────────────────────────────────────────
    // Helpers
    // ──────────────────────────────────────────────

    private int GetCurrentUserId()
    {
        var claim = User.FindFirst("userId")?.Value ?? User.FindFirst(ClaimTypes.NameIdentifier)?.Value;
        return int.TryParse(claim, out var id) ? id : 1;
    }

    private async Task<bool> IsAdminOrSelf(int targetUserId, CancellationToken ct)
    {
        var currentUserId = GetCurrentUserId();
        if (currentUserId == targetUserId) return true;
        return await _authService.IsAdminAsync(currentUserId, ct);
    }

    private static UserPreferences CreateDefaults(int userId) => new()
    {
        UserId = userId,
        NotificationsEnabled = true,
        CreatedAt = DateTime.UtcNow,
        UpdatedAt = DateTime.UtcNow
    };

    private static UserPreferencesResource ToResource(UserPreferences prefs)
    {
        List<MediaType>? hidden = null;
        if (!string.IsNullOrEmpty(prefs.HiddenMediaTypes))
        {
            try { hidden = JsonSerializer.Deserialize<List<int>>(prefs.HiddenMediaTypes)?.Select(i => (MediaType)i).ToList(); }
            catch { /* ignore */ }
        }

        Dictionary<string, int>? overrides = null;
        if (!string.IsNullOrEmpty(prefs.QualityProfileOverrides))
        {
            try { overrides = JsonSerializer.Deserialize<Dictionary<string, int>>(prefs.QualityProfileOverrides); }
            catch { /* ignore */ }
        }

        return new UserPreferencesResource
        {
            UserId = prefs.UserId,
            HiddenMediaTypes = hidden,
            DefaultQualityProfileId = prefs.DefaultQualityProfileId,
            QualityProfileOverrides = overrides,
            Language = prefs.Language,
            Theme = prefs.Theme,
            NotificationsEnabled = prefs.NotificationsEnabled
        };
    }

    private static SmartListSubscriptionResource ToResource(UserSmartListSubscription sub)
    {
        return new SmartListSubscriptionResource
        {
            Id = sub.Id,
            SmartListId = sub.SmartListId,
            AutoAdd = sub.AutoAdd,
            NotifyOnNew = sub.NotifyOnNew,
            SubscribedAt = sub.SubscribedAt
        };
    }
}

// ──────────────────────────────────────────────
// Resources
// ──────────────────────────────────────────────

public class UserPreferencesResource
{
    public int UserId { get; set; }
    public List<MediaType>? HiddenMediaTypes { get; set; }
    public int? DefaultQualityProfileId { get; set; }
    public Dictionary<string, int>? QualityProfileOverrides { get; set; }
    public string? Language { get; set; }
    public string? Theme { get; set; }
    public bool NotificationsEnabled { get; set; }
}

public class UpdatePreferencesRequest
{
    public List<MediaType>? HiddenMediaTypes { get; set; }
    public int? DefaultQualityProfileId { get; set; }
    public Dictionary<string, int>? QualityProfileOverrides { get; set; }
    public string? Language { get; set; }
    public string? Theme { get; set; }
    public bool? NotificationsEnabled { get; set; }
}

public class SmartListSubscriptionResource
{
    public int Id { get; set; }
    public int SmartListId { get; set; }
    public bool AutoAdd { get; set; }
    public bool NotifyOnNew { get; set; }
    public DateTime SubscribedAt { get; set; }
}

public class SubscribeRequest
{
    public bool AutoAdd { get; set; }
    public bool NotifyOnNew { get; set; } = true;
}

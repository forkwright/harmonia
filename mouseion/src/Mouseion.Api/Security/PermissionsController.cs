// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using System.Security.Claims;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Authentication;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Api.Security;

/// <summary>
/// Admin-only permission management, API key CRUD, audit log, and session management.
/// </summary>
[ApiController]
[Authorize]
[Route("api/v3")]
public class PermissionsController : ControllerBase
{
    private readonly Mouseion.Core.Authentication.IAuthorizationService _authService;
    private readonly IUserRepository _userRepository;

    public PermissionsController(
        Mouseion.Core.Authentication.IAuthorizationService authService,
        IUserRepository userRepository)
    {
        _authService = authService;
        _userRepository = userRepository;
    }

    // ──────────────────────────────────────────────
    // Permissions (admin only)
    // ──────────────────────────────────────────────

    /// <summary>Get a user's permissions.</summary>
    [HttpGet("users/{userId:int}/permissions")]
    public async Task<ActionResult<List<PermissionResource>>> GetPermissions(int userId, CancellationToken ct)
    {
        if (!await RequireAdmin(ct)) return Forbid();

        var permissions = await _authService.GetPermissionsAsync(userId, ct);
        return Ok(permissions.Select(ToResource).ToList());
    }

    /// <summary>Set a user's permissions (replaces all existing).</summary>
    [HttpPut("users/{userId:int}/permissions")]
    public async Task<ActionResult<List<PermissionResource>>> SetPermissions(
        int userId, [FromBody][Required] List<SetPermissionRequest> request, CancellationToken ct)
    {
        if (!await RequireAdmin(ct)) return Forbid();

        var permissions = request.Select(r => new UserPermission
        {
            PermissionType = r.PermissionType,
            ResourceId = r.ResourceId,
            Allowed = r.Allowed
        }).ToList();

        await _authService.SetPermissionsAsync(userId, permissions, GetCurrentUserId(), ct);

        var updated = await _authService.GetPermissionsAsync(userId, ct);
        return Ok(updated.Select(ToResource).ToList());
    }

    /// <summary>Get media types accessible to a user.</summary>
    [HttpGet("users/{userId:int}/accessible-media-types")]
    public async Task<ActionResult<List<string>>> GetAccessibleMediaTypes(int userId, CancellationToken ct)
    {
        if (!await RequireAdminOrSelf(userId, ct)) return Forbid();

        var types = await _authService.GetAccessibleMediaTypesAsync(userId, ct);
        return Ok(types.Select(t => t.ToString()).ToList());
    }

    // ──────────────────────────────────────────────
    // API Keys
    // ──────────────────────────────────────────────

    /// <summary>List current user's API keys.</summary>
    [HttpGet("apikeys")]
    public async Task<ActionResult<List<ApiKeyResource>>> GetMyApiKeys(CancellationToken ct)
    {
        var userId = GetCurrentUserId();
        var keys = await _authService.GetUserApiKeysAsync(userId, ct);
        return Ok(keys.Select(ToResource).ToList());
    }

    /// <summary>Create a new API key for the current user.</summary>
    [HttpPost("apikeys")]
    public async Task<ActionResult<ApiKeyCreatedResource>> CreateApiKey(
        [FromBody][Required] CreateApiKeyRequest request, CancellationToken ct)
    {
        var userId = GetCurrentUserId();
        var (key, rawKey) = await _authService.CreateApiKeyAsync(userId, request.Name, request.Scopes, request.ExpiresAt, ct);

        return CreatedAtAction(nameof(GetMyApiKeys), null, new ApiKeyCreatedResource
        {
            Id = key.Id,
            Name = key.Name,
            Key = rawKey, // Only time the full key is visible
            KeyPrefix = key.KeyPrefix,
            Scopes = request.Scopes,
            ExpiresAt = key.ExpiresAt,
            CreatedAt = key.CreatedAt
        });
    }

    /// <summary>Revoke an API key.</summary>
    [HttpDelete("apikeys/{keyId:int}")]
    public async Task<ActionResult> RevokeApiKey(int keyId, CancellationToken ct)
    {
        await _authService.RevokeApiKeyAsync(keyId, ct);
        return NoContent();
    }

    /// <summary>List a specific user's API keys (admin only).</summary>
    [HttpGet("users/{userId:int}/apikeys")]
    public async Task<ActionResult<List<ApiKeyResource>>> GetUserApiKeys(int userId, CancellationToken ct)
    {
        if (!await RequireAdmin(ct)) return Forbid();

        var keys = await _authService.GetUserApiKeysAsync(userId, ct);
        return Ok(keys.Select(ToResource).ToList());
    }

    // ──────────────────────────────────────────────
    // Audit Log (admin only)
    // ──────────────────────────────────────────────

    /// <summary>Get recent audit log entries.</summary>
    [HttpGet("admin/audit")]
    public async Task<ActionResult<List<AuditLogResource>>> GetAuditLog([FromQuery] int count = 100, CancellationToken ct = default)
    {
        if (!await RequireAdmin(ct)) return Forbid();

        var entries = await _authService.GetAuditLogAsync(count, ct);
        return Ok(entries.Select(ToResource).ToList());
    }

    /// <summary>Get audit log for a specific user.</summary>
    [HttpGet("admin/audit/user/{userId:int}")]
    public async Task<ActionResult<List<AuditLogResource>>> GetUserAuditLog(int userId, [FromQuery] int count = 50, CancellationToken ct = default)
    {
        if (!await RequireAdmin(ct)) return Forbid();

        var entries = await _authService.GetUserAuditLogAsync(userId, count, ct);
        return Ok(entries.Select(ToResource).ToList());
    }

    // ──────────────────────────────────────────────
    // Session Management (admin only)
    // ──────────────────────────────────────────────

    /// <summary>Get active sessions across all users.</summary>
    [HttpGet("admin/sessions")]
    public async Task<ActionResult<List<ActiveSessionResource>>> GetActiveSessions(CancellationToken ct)
    {
        if (!await RequireAdmin(ct)) return Forbid();

        var sessions = await _authService.GetActiveSessionsAsync(ct);
        return Ok(sessions.Select(s => new ActiveSessionResource
        {
            UserId = s.UserId,
            Username = s.Username,
            DisplayName = s.DisplayName,
            LastActivity = s.LastActivity,
            IpAddress = s.IpAddress,
            UserAgent = s.UserAgent,
            LoginCount = s.LoginCount
        }).ToList());
    }

    /// <summary>Revoke a user's session (force logout).</summary>
    [HttpDelete("admin/sessions/{userId:int}")]
    public async Task<ActionResult> RevokeSession(int userId, [FromQuery] string? sessionId = null, CancellationToken ct = default)
    {
        if (!await RequireAdmin(ct)) return Forbid();

        await _authService.RevokeSessionAsync(userId, sessionId ?? "all", ct);
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

    private async Task<bool> RequireAdmin(CancellationToken ct)
    {
        return await _authService.IsAdminAsync(GetCurrentUserId(), ct);
    }

    private async Task<bool> RequireAdminOrSelf(int targetUserId, CancellationToken ct)
    {
        var currentUserId = GetCurrentUserId();
        if (currentUserId == targetUserId) return true;
        return await _authService.IsAdminAsync(currentUserId, ct);
    }

    // ──────────────────────────────────────────────
    // Resource mappers
    // ──────────────────────────────────────────────

    private static PermissionResource ToResource(UserPermission p) => new()
    {
        Id = p.Id,
        PermissionType = p.PermissionType.ToString(),
        ResourceId = p.ResourceId,
        Allowed = p.Allowed,
        GrantedBy = p.GrantedBy,
        GrantedAt = p.GrantedAt
    };

    private static ApiKeyResource ToResource(ApiKey k) => new()
    {
        Id = k.Id,
        Name = k.Name,
        KeyPrefix = k.KeyPrefix,
        Scopes = !string.IsNullOrEmpty(k.Scopes)
            ? global::System.Text.Json.JsonSerializer.Deserialize<List<string>>(k.Scopes)
            : null,
        ExpiresAt = k.ExpiresAt,
        LastUsedAt = k.LastUsedAt,
        IsActive = k.IsActive,
        CreatedAt = k.CreatedAt
    };

    private static AuditLogResource ToResource(AuditLogEntry e) => new()
    {
        Id = e.Id,
        UserId = e.UserId,
        Action = e.Action,
        ResourceType = e.ResourceType,
        ResourceId = e.ResourceId,
        Details = e.Details,
        IpAddress = e.IpAddress,
        Timestamp = e.Timestamp
    };
}

// ──────────────────────────────────────────────
// Resources
// ──────────────────────────────────────────────

public class PermissionResource
{
    public int Id { get; set; }
    public string PermissionType { get; set; } = string.Empty;
    public string ResourceId { get; set; } = string.Empty;
    public bool Allowed { get; set; }
    public int? GrantedBy { get; set; }
    public DateTime GrantedAt { get; set; }
}

public class SetPermissionRequest
{
    public PermissionType PermissionType { get; set; }
    public string ResourceId { get; set; } = string.Empty;
    public bool Allowed { get; set; } = true;
}

public class ApiKeyResource
{
    public int Id { get; set; }
    public string Name { get; set; } = string.Empty;
    public string KeyPrefix { get; set; } = string.Empty;
    public List<string>? Scopes { get; set; }
    public DateTime? ExpiresAt { get; set; }
    public DateTime? LastUsedAt { get; set; }
    public bool IsActive { get; set; }
    public DateTime CreatedAt { get; set; }
}

public class ApiKeyCreatedResource
{
    public int Id { get; set; }
    public string Name { get; set; } = string.Empty;
    public string Key { get; set; } = string.Empty; // Full key — only shown at creation
    public string KeyPrefix { get; set; } = string.Empty;
    public List<string>? Scopes { get; set; }
    public DateTime? ExpiresAt { get; set; }
    public DateTime CreatedAt { get; set; }
}

public class CreateApiKeyRequest
{
    public string Name { get; set; } = string.Empty;
    public List<string>? Scopes { get; set; }
    public DateTime? ExpiresAt { get; set; }
}

public class AuditLogResource
{
    public int Id { get; set; }
    public int? UserId { get; set; }
    public string Action { get; set; } = string.Empty;
    public string? ResourceType { get; set; }
    public string? ResourceId { get; set; }
    public string? Details { get; set; }
    public string? IpAddress { get; set; }
    public DateTime Timestamp { get; set; }
}

public class ActiveSessionResource
{
    public int UserId { get; set; }
    public string Username { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public DateTime LastActivity { get; set; }
    public string? IpAddress { get; set; }
    public string? UserAgent { get; set; }
    public int LoginCount { get; set; }
}

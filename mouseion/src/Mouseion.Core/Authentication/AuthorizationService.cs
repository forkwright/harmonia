// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Authentication;

/// <summary>
/// Centralized authorization: checks user permissions, manages API keys,
/// and records audit events. All permission checks go through here.
/// </summary>
public interface IAuthorizationService
{
    // Permission checks
    Task<bool> CanAccessMediaTypeAsync(int userId, MediaType mediaType, CancellationToken ct = default);
    Task<bool> CanAccessRootFolderAsync(int userId, string rootFolderPath, CancellationToken ct = default);
    Task<bool> CanDownloadAsync(int userId, CancellationToken ct = default);
    Task<bool> IsAdminAsync(int userId, CancellationToken ct = default);
    Task<List<MediaType>> GetAccessibleMediaTypesAsync(int userId, CancellationToken ct = default);

    // Permission management (admin only)
    Task GrantMediaTypeAccessAsync(int userId, MediaType mediaType, int grantedBy, CancellationToken ct = default);
    Task RevokeMediaTypeAccessAsync(int userId, MediaType mediaType, CancellationToken ct = default);
    Task SetPermissionsAsync(int userId, List<UserPermission> permissions, int grantedBy, CancellationToken ct = default);
    Task<List<UserPermission>> GetPermissionsAsync(int userId, CancellationToken ct = default);

    // API key management
    Task<(ApiKey key, string rawKey)> CreateApiKeyAsync(int userId, string name, List<string>? scopes = null, DateTime? expiresAt = null, CancellationToken ct = default);
    Task<ApiKey?> ValidateApiKeyAsync(string rawKey, CancellationToken ct = default);
    Task RevokeApiKeyAsync(int keyId, CancellationToken ct = default);
    Task<List<ApiKey>> GetUserApiKeysAsync(int userId, CancellationToken ct = default);

    // Audit
    Task LogAsync(AuditLogEntry entry, CancellationToken ct = default);
    Task LogAsync(int? userId, string action, string? resourceType = null, string? resourceId = null, string? details = null, string? ipAddress = null, CancellationToken ct = default);
    Task<List<AuditLogEntry>> GetAuditLogAsync(int count = 100, CancellationToken ct = default);
    Task<List<AuditLogEntry>> GetUserAuditLogAsync(int userId, int count = 50, CancellationToken ct = default);

    // Session management
    Task<List<ActiveSession>> GetActiveSessionsAsync(CancellationToken ct = default);
    Task RevokeSessionAsync(int userId, string sessionId, CancellationToken ct = default);
}

public class AuthorizationService : IAuthorizationService
{
    private readonly IUserRepository _userRepository;
    private readonly IUserPermissionRepository _permissionRepository;
    private readonly IApiKeyRepository _apiKeyRepository;
    private readonly IAuditLogRepository _auditLogRepository;
    private readonly ILogger<AuthorizationService> _logger;

    public AuthorizationService(
        IUserRepository userRepository,
        IUserPermissionRepository permissionRepository,
        IApiKeyRepository apiKeyRepository,
        IAuditLogRepository auditLogRepository,
        ILogger<AuthorizationService> logger)
    {
        _userRepository = userRepository;
        _permissionRepository = permissionRepository;
        _apiKeyRepository = apiKeyRepository;
        _auditLogRepository = auditLogRepository;
        _logger = logger;
    }

    // ──────────────────────────────────────────────
    // Permission checks
    // ──────────────────────────────────────────────

    public async Task<bool> CanAccessMediaTypeAsync(int userId, MediaType mediaType, CancellationToken ct = default)
    {
        var user = _userRepository.Get(userId);
        if (user.Role == UserRole.Admin) return true; // Admins bypass all checks

        var permissions = await _permissionRepository.GetByTypeAsync(userId, PermissionType.MediaTypeAccess, ct);

        // If no media type permissions are set, allow all (default-open)
        if (permissions.Count == 0) return true;

        return permissions.Any(p =>
            p.ResourceId == ((int)mediaType).ToString() && p.Allowed);
    }

    public async Task<bool> CanAccessRootFolderAsync(int userId, string rootFolderPath, CancellationToken ct = default)
    {
        var user = _userRepository.Get(userId);
        if (user.Role == UserRole.Admin) return true;

        var permissions = await _permissionRepository.GetByTypeAsync(userId, PermissionType.RootFolderAccess, ct);

        // If no root folder permissions are set, allow all
        if (permissions.Count == 0) return true;

        return permissions.Any(p =>
            rootFolderPath.StartsWith(p.ResourceId, StringComparison.OrdinalIgnoreCase) && p.Allowed);
    }

    public async Task<bool> CanDownloadAsync(int userId, CancellationToken ct = default)
    {
        var user = _userRepository.Get(userId);
        if (user.Role == UserRole.Admin) return true;
        if (user.Role == UserRole.ReadOnly) return false;

        var permissions = await _permissionRepository.GetByTypeAsync(userId, PermissionType.DownloadPermission, ct);

        // Default: User role can download unless explicitly denied
        if (permissions.Count == 0) return user.Role >= UserRole.User;

        return permissions.Any(p => p.ResourceId == "download" && p.Allowed);
    }

    public async Task<bool> IsAdminAsync(int userId, CancellationToken ct = default)
    {
        var user = _userRepository.Get(userId);
        return user.Role == UserRole.Admin;
    }

    public async Task<List<MediaType>> GetAccessibleMediaTypesAsync(int userId, CancellationToken ct = default)
    {
        var user = _userRepository.Get(userId);
        var allTypes = Enum.GetValues<MediaType>().Where(t => t != MediaType.Unknown).ToList();

        if (user.Role == UserRole.Admin) return allTypes;

        var permissions = await _permissionRepository.GetByTypeAsync(userId, PermissionType.MediaTypeAccess, ct);
        if (permissions.Count == 0) return allTypes;

        return permissions
            .Where(p => p.Allowed && int.TryParse(p.ResourceId, out var typeId) && Enum.IsDefined(typeof(MediaType), typeId))
            .Select(p => (MediaType)int.Parse(p.ResourceId))
            .ToList();
    }

    // ──────────────────────────────────────────────
    // Permission management
    // ──────────────────────────────────────────────

    public async Task GrantMediaTypeAccessAsync(int userId, MediaType mediaType, int grantedBy, CancellationToken ct = default)
    {
        var permission = new UserPermission
        {
            UserId = userId,
            PermissionType = PermissionType.MediaTypeAccess,
            ResourceId = ((int)mediaType).ToString(),
            Allowed = true,
            GrantedBy = grantedBy,
            GrantedAt = DateTime.UtcNow
        };

        _permissionRepository.Insert(permission);

        await LogAsync(grantedBy, "permission_grant",
            "MediaType", mediaType.ToString(),
            JsonSerializer.Serialize(new { targetUser = userId, mediaType = mediaType.ToString() }),
            ct: ct);
    }

    public async Task RevokeMediaTypeAccessAsync(int userId, MediaType mediaType, CancellationToken ct = default)
    {
        var permissions = await _permissionRepository.GetByTypeAsync(userId, PermissionType.MediaTypeAccess, ct);
        var match = permissions.FirstOrDefault(p => p.ResourceId == ((int)mediaType).ToString());
        if (match != null)
        {
            _permissionRepository.Delete(match.Id);
        }
    }

    public async Task SetPermissionsAsync(int userId, List<UserPermission> permissions, int grantedBy, CancellationToken ct = default)
    {
        // Replace all permissions for user
        await _permissionRepository.DeleteAllForUserAsync(userId, ct);

        foreach (var perm in permissions)
        {
            perm.UserId = userId;
            perm.GrantedBy = grantedBy;
            perm.GrantedAt = DateTime.UtcNow;
            _permissionRepository.Insert(perm);
        }

        await LogAsync(grantedBy, "permissions_set",
            "User", userId.ToString(),
            JsonSerializer.Serialize(new { permissionCount = permissions.Count }),
            ct: ct);
    }

    public async Task<List<UserPermission>> GetPermissionsAsync(int userId, CancellationToken ct = default)
    {
        return await _permissionRepository.GetByUserIdAsync(userId, ct);
    }

    // ──────────────────────────────────────────────
    // API key management
    // ──────────────────────────────────────────────

    public async Task<(ApiKey key, string rawKey)> CreateApiKeyAsync(int userId, string name, List<string>? scopes = null, DateTime? expiresAt = null, CancellationToken ct = default)
    {
        // Generate a cryptographically random key
        var rawKey = GenerateApiKey();
        var prefix = rawKey[..8];

        var apiKey = new ApiKey
        {
            UserId = userId,
            Name = name,
            KeyHash = HashApiKey(rawKey),
            KeyPrefix = prefix,
            Scopes = scopes != null ? JsonSerializer.Serialize(scopes) : null,
            ExpiresAt = expiresAt,
            CreatedAt = DateTime.UtcNow
        };

        var inserted = _apiKeyRepository.Insert(apiKey);

        await LogAsync(userId, "apikey_created",
            "ApiKey", inserted.Id.ToString(),
            JsonSerializer.Serialize(new { name, prefix, scopes }),
            ct: ct);

        _logger.LogInformation("API key created: {Name} ({Prefix}...) for user {UserId}", name, prefix, userId);

        // Return both the key entity and the raw key (only time it's visible)
        return (inserted, rawKey);
    }

    public async Task<ApiKey?> ValidateApiKeyAsync(string rawKey, CancellationToken ct = default)
    {
        if (string.IsNullOrEmpty(rawKey) || rawKey.Length < 8) return null;

        var prefix = rawKey[..8];
        var apiKey = await _apiKeyRepository.GetByPrefixAsync(prefix, ct);

        if (apiKey == null) return null;

        // Check expiration
        if (apiKey.ExpiresAt.HasValue && apiKey.ExpiresAt.Value < DateTime.UtcNow)
        {
            _logger.LogDebug("API key {Prefix}... expired", prefix);
            return null;
        }

        // Verify hash
        if (!VerifyApiKey(rawKey, apiKey.KeyHash))
        {
            _logger.LogDebug("API key {Prefix}... hash mismatch", prefix);
            return null;
        }

        // Update last used
        await _apiKeyRepository.UpdateLastUsedAsync(apiKey.Id, ct);

        return apiKey;
    }

    public async Task RevokeApiKeyAsync(int keyId, CancellationToken ct = default)
    {
        var key = _apiKeyRepository.Get(keyId);
        key.IsActive = false;
        _apiKeyRepository.Update(key);

        await LogAsync(key.UserId, "apikey_revoked",
            "ApiKey", keyId.ToString(),
            JsonSerializer.Serialize(new { name = key.Name, prefix = key.KeyPrefix }),
            ct: ct);
    }

    public async Task<List<ApiKey>> GetUserApiKeysAsync(int userId, CancellationToken ct = default)
    {
        return await _apiKeyRepository.GetByUserIdAsync(userId, ct);
    }

    // ──────────────────────────────────────────────
    // Audit
    // ──────────────────────────────────────────────

    public async Task LogAsync(AuditLogEntry entry, CancellationToken ct = default)
    {
        entry.Timestamp = DateTime.UtcNow;
        await _auditLogRepository.InsertAsync(entry, ct);
    }

    public async Task LogAsync(int? userId, string action, string? resourceType = null, string? resourceId = null, string? details = null, string? ipAddress = null, CancellationToken ct = default)
    {
        await _auditLogRepository.InsertAsync(new AuditLogEntry
        {
            UserId = userId,
            Action = action,
            ResourceType = resourceType,
            ResourceId = resourceId,
            Details = details,
            IpAddress = ipAddress,
            Timestamp = DateTime.UtcNow
        }, ct);
    }

    public async Task<List<AuditLogEntry>> GetAuditLogAsync(int count = 100, CancellationToken ct = default)
    {
        return await _auditLogRepository.GetRecentAsync(count, ct);
    }

    public async Task<List<AuditLogEntry>> GetUserAuditLogAsync(int userId, int count = 50, CancellationToken ct = default)
    {
        return await _auditLogRepository.GetByUserIdAsync(userId, count, ct);
    }

    // ──────────────────────────────────────────────
    // Session management
    // ──────────────────────────────────────────────

    public async Task<List<ActiveSession>> GetActiveSessionsAsync(CancellationToken ct = default)
    {
        // Active sessions tracked via audit log login events + refresh tokens
        var logins = await _auditLogRepository.GetByActionAsync("login", 200, ct);
        var users = await _userRepository.GetActiveUsersAsync(ct);
        var userMap = users.ToDictionary(u => u.Id, u => u);

        return logins
            .Where(l => l.UserId.HasValue && userMap.ContainsKey(l.UserId!.Value))
            .GroupBy(l => l.UserId!.Value)
            .Select(g =>
            {
                var latest = g.OrderByDescending(l => l.Timestamp).First();
                var user = userMap[g.Key];
                return new ActiveSession
                {
                    UserId = g.Key,
                    Username = user.Username,
                    DisplayName = user.DisplayName,
                    LastActivity = latest.Timestamp,
                    IpAddress = latest.IpAddress,
                    UserAgent = latest.UserAgent,
                    LoginCount = g.Count()
                };
            })
            .OrderByDescending(s => s.LastActivity)
            .ToList();
    }

    public async Task RevokeSessionAsync(int userId, string sessionId, CancellationToken ct = default)
    {
        // Log the revocation — actual token invalidation depends on JWT blacklist or short expiry
        await LogAsync(userId, "session_revoked",
            "Session", sessionId,
            ct: ct);

        _logger.LogInformation("Session revoked for user {UserId}: {SessionId}", userId, sessionId);
    }

    // ──────────────────────────────────────────────
    // Helpers
    // ──────────────────────────────────────────────

    private static string GenerateApiKey()
    {
        var bytes = new byte[32];
        using var rng = RandomNumberGenerator.Create();
        rng.GetBytes(bytes);
        return Convert.ToBase64String(bytes).Replace("+", "-").Replace("/", "_").TrimEnd('=');
    }

    private static string HashApiKey(string rawKey)
    {
        // PBKDF2 matching AuthenticationService's pattern
        var salt = RandomNumberGenerator.GetBytes(16);
        var hash = Rfc2898DeriveBytes.Pbkdf2(rawKey, salt, 100_000, HashAlgorithmName.SHA512, 32);
        return $"100000.SHA512.{Convert.ToBase64String(salt)}.{Convert.ToBase64String(hash)}";
    }

    private static bool VerifyApiKey(string rawKey, string storedHash)
    {
        try
        {
            var parts = storedHash.Split('.');
            if (parts.Length != 4) return false;

            var iterations = int.Parse(parts[0]);
            var algorithm = new HashAlgorithmName(parts[1]);
            var salt = Convert.FromBase64String(parts[2]);
            var hash = Convert.FromBase64String(parts[3]);

            var computedHash = Rfc2898DeriveBytes.Pbkdf2(rawKey, salt, iterations, algorithm, hash.Length);
            return CryptographicOperations.FixedTimeEquals(computedHash, hash);
        }
        catch
        {
            return false;
        }
    }
}

public class ActiveSession
{
    public int UserId { get; set; }
    public string Username { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public DateTime LastActivity { get; set; }
    public string? IpAddress { get; set; }
    public string? UserAgent { get; set; }
    public int LoginCount { get; set; }
}

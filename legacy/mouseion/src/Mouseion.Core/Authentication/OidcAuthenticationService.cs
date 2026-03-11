// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Collections.Concurrent;
using System.Net.Http.Json;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Serilog;

namespace Mouseion.Core.Authentication;

public interface IOidcAuthenticationService
{
    /// <summary>Get all enabled providers for the login UI</summary>
    Task<List<OidcProvider>> GetEnabledProvidersAsync(CancellationToken ct = default);

    /// <summary>Generate authorization URL with PKCE for a provider</summary>
    Task<OidcAuthorizeResult> GenerateAuthorizeUrlAsync(string slug, string redirectUri, CancellationToken ct = default);

    /// <summary>Handle the callback from the OIDC provider — exchange code, provision/find user</summary>
    Task<OidcCallbackResult> HandleCallbackAsync(string state, string code, CancellationToken ct = default);

    /// <summary>Fetch and cache the discovery document for a provider</summary>
    Task<OidcDiscoveryDocument> GetDiscoveryDocumentAsync(OidcProvider provider, CancellationToken ct = default);
}

public class OidcAuthorizeResult
{
    public string AuthorizeUrl { get; set; } = string.Empty;
    public string State { get; set; } = string.Empty;
}

public class OidcCallbackResult
{
    public bool Success { get; set; }
    public User? User { get; set; }
    public string? Error { get; set; }
    public bool IsNewUser { get; set; }
}

public class OidcAuthenticationService : IOidcAuthenticationService
{
    private readonly IOidcProviderRepository _providerRepo;
    private readonly IOidcAuthStateRepository _authStateRepo;
    private readonly IUserRepository _userRepo;
    private readonly IHttpClientFactory _httpClientFactory;
    private static readonly ILogger Logger = Log.ForContext<OidcAuthenticationService>();

    // Cache discovery documents for 1 hour
    private readonly ConcurrentDictionary<string, (OidcDiscoveryDocument Doc, DateTime ExpiresAt)> _discoveryCache = new();

    public OidcAuthenticationService(
        IOidcProviderRepository providerRepo,
        IOidcAuthStateRepository authStateRepo,
        IUserRepository userRepo,
        IHttpClientFactory httpClientFactory)
    {
        _providerRepo = providerRepo;
        _authStateRepo = authStateRepo;
        _userRepo = userRepo;
        _httpClientFactory = httpClientFactory;
    }

    public async Task<List<OidcProvider>> GetEnabledProvidersAsync(CancellationToken ct = default)
    {
        return await _providerRepo.GetEnabledAsync(ct).ConfigureAwait(false);
    }

    public async Task<OidcAuthorizeResult> GenerateAuthorizeUrlAsync(string slug, string redirectUri, CancellationToken ct = default)
    {
        var provider = await _providerRepo.GetBySlugAsync(slug, ct).ConfigureAwait(false)
            ?? throw new InvalidOperationException($"OIDC provider '{slug}' not found");

        if (!provider.Enabled)
            throw new InvalidOperationException($"OIDC provider '{slug}' is disabled");

        var discovery = await GetDiscoveryDocumentAsync(provider, ct).ConfigureAwait(false);

        // Generate PKCE code verifier and challenge
        var codeVerifier = GenerateCodeVerifier();
        var codeChallenge = GenerateCodeChallenge(codeVerifier);
        var state = GenerateState();

        // Store auth state for callback validation
        var authState = new OidcAuthState
        {
            ProviderId = provider.Id,
            State = state,
            CodeVerifier = codeVerifier,
            RedirectUri = redirectUri,
            ExpiresAt = DateTime.UtcNow.AddMinutes(10),
            CreatedAt = DateTime.UtcNow
        };

        await _authStateRepo.InsertAsync(authState, ct).ConfigureAwait(false);

        // Build authorization URL
        var authorizeUrl = $"{discovery.AuthorizationEndpoint}" +
            $"?client_id={Uri.EscapeDataString(provider.ClientId)}" +
            $"&response_type=code" +
            $"&scope={Uri.EscapeDataString(provider.Scopes)}" +
            $"&redirect_uri={Uri.EscapeDataString(redirectUri)}" +
            $"&state={Uri.EscapeDataString(state)}" +
            $"&code_challenge={Uri.EscapeDataString(codeChallenge)}" +
            $"&code_challenge_method=S256";

        Logger.Information("OIDC authorize URL generated for provider {Provider}", provider.Name);

        return new OidcAuthorizeResult { AuthorizeUrl = authorizeUrl, State = state };
    }

    public async Task<OidcCallbackResult> HandleCallbackAsync(string state, string code, CancellationToken ct = default)
    {
        // Validate state and retrieve PKCE verifier
        var authState = await _authStateRepo.GetByStateAsync(state, ct).ConfigureAwait(false);
        if (authState == null)
        {
            Logger.Warning("OIDC callback with invalid or expired state");
            return new OidcCallbackResult { Success = false, Error = "Invalid or expired state parameter" };
        }

        // Clean up used state
        await _authStateRepo.DeleteAsync(authState.Id, ct).ConfigureAwait(false);

        var provider = await _providerRepo.FindAsync(authState.ProviderId, ct).ConfigureAwait(false);
        if (provider == null || !provider.Enabled)
        {
            return new OidcCallbackResult { Success = false, Error = "OIDC provider not found or disabled" };
        }

        try
        {
            var discovery = await GetDiscoveryDocumentAsync(provider, ct).ConfigureAwait(false);

            // Exchange authorization code for tokens
            var tokenResponse = await ExchangeCodeForTokensAsync(
                discovery.TokenEndpoint, provider, code,
                authState.RedirectUri, authState.CodeVerifier, ct).ConfigureAwait(false);

            if (tokenResponse == null || string.IsNullOrEmpty(tokenResponse.AccessToken))
            {
                return new OidcCallbackResult { Success = false, Error = "Failed to exchange authorization code" };
            }

            // Fetch user info from the provider
            var userInfo = await FetchUserInfoAsync(
                discovery.UserinfoEndpoint, tokenResponse.AccessToken, ct).ConfigureAwait(false);

            if (userInfo == null || string.IsNullOrEmpty(userInfo.Subject))
            {
                return new OidcCallbackResult { Success = false, Error = "Failed to fetch user info from provider" };
            }

            // Find or create user
            var (user, isNew) = await FindOrCreateUserAsync(provider, userInfo, ct).ConfigureAwait(false);

            Logger.Information("OIDC login successful: {Username} via {Provider} (new: {IsNew})",
                user.Username, provider.Name, isNew);

            return new OidcCallbackResult { Success = true, User = user, IsNewUser = isNew };
        }
        catch (Exception ex)
        {
            Logger.Error(ex, "OIDC callback failed for provider {Provider}", provider.Name);
            return new OidcCallbackResult { Success = false, Error = $"Authentication failed: {ex.Message}" };
        }
    }

    public async Task<OidcDiscoveryDocument> GetDiscoveryDocumentAsync(OidcProvider provider, CancellationToken ct = default)
    {
        var cacheKey = provider.IssuerUrl;

        if (_discoveryCache.TryGetValue(cacheKey, out var cached) && cached.ExpiresAt > DateTime.UtcNow)
        {
            return cached.Doc;
        }

        var discoveryUrl = provider.IssuerUrl.TrimEnd('/') + "/.well-known/openid-configuration";
        var client = _httpClientFactory.CreateClient("OidcDiscovery");

        var response = await client.GetAsync(discoveryUrl, ct).ConfigureAwait(false);
        response.EnsureSuccessStatusCode();

        var json = await response.Content.ReadFromJsonAsync<JsonElement>(cancellationToken: ct).ConfigureAwait(false);

        var doc = new OidcDiscoveryDocument
        {
            AuthorizationEndpoint = json.GetProperty("authorization_endpoint").GetString() ?? string.Empty,
            TokenEndpoint = json.GetProperty("token_endpoint").GetString() ?? string.Empty,
            UserinfoEndpoint = json.TryGetProperty("userinfo_endpoint", out var ui) ? ui.GetString() ?? string.Empty : string.Empty,
            JwksUri = json.TryGetProperty("jwks_uri", out var jwks) ? jwks.GetString() ?? string.Empty : string.Empty,
            EndSessionEndpoint = json.TryGetProperty("end_session_endpoint", out var es) ? es.GetString() ?? string.Empty : string.Empty,
            FetchedAt = DateTime.UtcNow
        };

        _discoveryCache[cacheKey] = (doc, DateTime.UtcNow.AddHours(1));

        Logger.Information("OIDC discovery document fetched for {Issuer}", provider.IssuerUrl);
        return doc;
    }

    private async Task<OidcTokenResponse?> ExchangeCodeForTokensAsync(
        string tokenEndpoint, OidcProvider provider, string code,
        string redirectUri, string codeVerifier, CancellationToken ct)
    {
        var client = _httpClientFactory.CreateClient("OidcToken");

        var content = new FormUrlEncodedContent(new Dictionary<string, string>
        {
            ["grant_type"] = "authorization_code",
            ["client_id"] = provider.ClientId,
            ["client_secret"] = provider.ClientSecret,
            ["code"] = code,
            ["redirect_uri"] = redirectUri,
            ["code_verifier"] = codeVerifier
        });

        var response = await client.PostAsync(tokenEndpoint, content, ct).ConfigureAwait(false);

        if (!response.IsSuccessStatusCode)
        {
            var errorBody = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
            Logger.Warning("OIDC token exchange failed: {StatusCode} {Body}", response.StatusCode, errorBody);
            return null;
        }

        return await response.Content.ReadFromJsonAsync<OidcTokenResponse>(cancellationToken: ct).ConfigureAwait(false);
    }

    private async Task<OidcUserInfo?> FetchUserInfoAsync(string userinfoEndpoint, string accessToken, CancellationToken ct)
    {
        if (string.IsNullOrEmpty(userinfoEndpoint))
            return null;

        var client = _httpClientFactory.CreateClient("OidcUserInfo");
        client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", accessToken);

        var response = await client.GetAsync(userinfoEndpoint, ct).ConfigureAwait(false);
        if (!response.IsSuccessStatusCode)
            return null;

        var json = await response.Content.ReadFromJsonAsync<JsonElement>(cancellationToken: ct).ConfigureAwait(false);

        return new OidcUserInfo
        {
            Subject = json.TryGetProperty("sub", out var sub) ? sub.GetString() ?? string.Empty : string.Empty,
            PreferredUsername = json.TryGetProperty("preferred_username", out var pref) ? pref.GetString() : null,
            Email = json.TryGetProperty("email", out var email) ? email.GetString() : null,
            Name = json.TryGetProperty("name", out var name) ? name.GetString() : null,
            Roles = ExtractRoles(json)
        };
    }

    private static List<string> ExtractRoles(JsonElement json)
    {
        var roles = new List<string>();

        // Try common claim locations
        foreach (var claimName in new[] { "roles", "groups", "realm_access" })
        {
            if (json.TryGetProperty(claimName, out var claimValue))
            {
                if (claimValue.ValueKind == JsonValueKind.Array)
                {
                    foreach (var item in claimValue.EnumerateArray())
                    {
                        if (item.ValueKind == JsonValueKind.String)
                            roles.Add(item.GetString()!);
                    }
                }
                else if (claimValue.ValueKind == JsonValueKind.Object && claimValue.TryGetProperty("roles", out var nestedRoles))
                {
                    foreach (var item in nestedRoles.EnumerateArray())
                    {
                        if (item.ValueKind == JsonValueKind.String)
                            roles.Add(item.GetString()!);
                    }
                }
            }
        }

        return roles;
    }

    private async Task<(User User, bool IsNew)> FindOrCreateUserAsync(
        OidcProvider provider, OidcUserInfo userInfo, CancellationToken ct)
    {
        // Try to find existing user by OIDC subject
        var existingUser = await FindUserByOidcSubjectAsync(provider.Id, userInfo.Subject, ct).ConfigureAwait(false);
        if (existingUser != null)
        {
            await _userRepo.UpdateLastLoginAsync(existingUser.Id, ct).ConfigureAwait(false);
            return (existingUser, false);
        }

        // Try to link by email
        if (!string.IsNullOrEmpty(userInfo.Email))
        {
            var emailUser = await _userRepo.GetByEmailAsync(userInfo.Email, ct).ConfigureAwait(false);
            if (emailUser != null)
            {
                emailUser.OidcProviderId = provider.Id;
                emailUser.OidcSubject = userInfo.Subject;
                emailUser.AuthenticationMethod = "oidc";
                emailUser.UpdatedAt = DateTime.UtcNow;
                await _userRepo.UpdateAsync(emailUser, ct).ConfigureAwait(false);
                await _userRepo.UpdateLastLoginAsync(emailUser.Id, ct).ConfigureAwait(false);
                Logger.Information("Linked OIDC subject to existing user {Username} via email match", emailUser.Username);
                return (emailUser, false);
            }
        }

        // Auto-provision new user
        if (!provider.AutoProvisionUsers)
        {
            throw new UnauthorizedAccessException(
                $"User not found and auto-provisioning is disabled for provider '{provider.Name}'");
        }

        var role = ResolveRole(provider, userInfo.Roles);
        var username = userInfo.PreferredUsername ?? userInfo.Email ?? $"oidc_{userInfo.Subject[..8]}";

        // Ensure unique username
        if (await _userRepo.UsernameExistsAsync(username, ct).ConfigureAwait(false))
        {
            username = $"{username}_{provider.Slug}";
        }

        var newUser = new User
        {
            Username = username,
            DisplayName = userInfo.Name ?? username,
            Email = userInfo.Email ?? string.Empty,
            Role = role,
            AuthenticationMethod = "oidc",
            PasswordHash = string.Empty, // No local password for OIDC users
            IsActive = true,
            OidcProviderId = provider.Id,
            OidcSubject = userInfo.Subject,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow
        };

        var created = await _userRepo.InsertAsync(newUser, ct).ConfigureAwait(false);
        Logger.Information("Auto-provisioned OIDC user: {Username} (provider: {Provider}, role: {Role})",
            created.Username, provider.Name, created.Role);

        return (created, true);
    }

    private async Task<User?> FindUserByOidcSubjectAsync(int providerId, string subject, CancellationToken ct)
    {
        return await _userRepo.GetByOidcSubjectAsync(providerId, subject, ct).ConfigureAwait(false);
    }

    private UserRole ResolveRole(OidcProvider provider, List<string> userRoles)
    {
        if (string.IsNullOrEmpty(provider.ClaimRoleMapping) || provider.ClaimRoleMapping == "{}")
            return provider.DefaultRole;

        try
        {
            var mapping = JsonSerializer.Deserialize<Dictionary<string, string>>(provider.ClaimRoleMapping);
            if (mapping == null) return provider.DefaultRole;

            foreach (var role in userRoles)
            {
                if (mapping.TryGetValue(role, out var mouseionRole) &&
                    Enum.TryParse<UserRole>(mouseionRole, true, out var parsed))
                {
                    return parsed;
                }
            }
        }
        catch (JsonException ex)
        {
            Logger.Warning(ex, "Failed to parse ClaimRoleMapping for provider {Provider}", provider.Name);
        }

        return provider.DefaultRole;
    }

    private static string GenerateCodeVerifier()
    {
        var bytes = RandomNumberGenerator.GetBytes(32);
        return Base64UrlEncode(bytes);
    }

    private static string GenerateCodeChallenge(string codeVerifier)
    {
        var bytes = SHA256.HashData(Encoding.ASCII.GetBytes(codeVerifier));
        return Base64UrlEncode(bytes);
    }

    private static string GenerateState()
    {
        return Base64UrlEncode(RandomNumberGenerator.GetBytes(32));
    }

    private static string Base64UrlEncode(byte[] bytes)
    {
        return Convert.ToBase64String(bytes)
            .Replace('+', '-')
            .Replace('/', '_')
            .TrimEnd('=');
    }
}

internal class OidcTokenResponse
{
    public string AccessToken { get; set; } = string.Empty;
    public string? IdToken { get; set; }
    public string? RefreshToken { get; set; }
    public string TokenType { get; set; } = "Bearer";
    public int ExpiresIn { get; set; }
}

internal class OidcUserInfo
{
    public string Subject { get; set; } = string.Empty;
    public string? PreferredUsername { get; set; }
    public string? Email { get; set; }
    public string? Name { get; set; }
    public List<string> Roles { get; set; } = new();
}

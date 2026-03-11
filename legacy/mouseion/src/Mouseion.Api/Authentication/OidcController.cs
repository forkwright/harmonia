// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Authentication;

namespace Mouseion.Api.Authentication;

[ApiController]
[Route("api/v3/auth/oidc")]
public class OidcController : ControllerBase
{
    private readonly IOidcAuthenticationService _oidcService;
    private readonly IOidcProviderRepository _providerRepo;
    private readonly IJwtTokenService _jwtTokenService;

    public OidcController(
        IOidcAuthenticationService oidcService,
        IOidcProviderRepository providerRepo,
        IJwtTokenService jwtTokenService)
    {
        _oidcService = oidcService;
        _providerRepo = providerRepo;
        _jwtTokenService = jwtTokenService;
    }

    /// <summary>Get enabled OIDC providers for the login page</summary>
    [HttpGet("providers")]
    [AllowAnonymous]
    public async Task<ActionResult<List<OidcProviderInfo>>> GetProviders(CancellationToken ct = default)
    {
        var providers = await _oidcService.GetEnabledProvidersAsync(ct).ConfigureAwait(false);
        return Ok(providers.Select(p => new OidcProviderInfo
        {
            Slug = p.Slug,
            Name = p.Name,
            AuthorizeUrl = $"/api/v3/auth/oidc/{p.Slug}/authorize"
        }).ToList());
    }

    /// <summary>Initiate OIDC authorization code flow with PKCE</summary>
    [HttpGet("{slug}/authorize")]
    [AllowAnonymous]
    public async Task<ActionResult> Authorize(
        string slug,
        [FromQuery] string? redirect_uri = null,
        CancellationToken ct = default)
    {
        try
        {
            var callbackUri = redirect_uri ?? $"{Request.Scheme}://{Request.Host}/api/v3/auth/oidc/callback";

            var result = await _oidcService.GenerateAuthorizeUrlAsync(slug, callbackUri, ct)
                .ConfigureAwait(false);

            return Redirect(result.AuthorizeUrl);
        }
        catch (InvalidOperationException ex)
        {
            return NotFound(new { error = ex.Message });
        }
    }

    /// <summary>Handle OIDC provider callback — exchange code, issue JWT</summary>
    [HttpGet("callback")]
    [AllowAnonymous]
    public async Task<ActionResult<LoginResponse>> Callback(
        [FromQuery] string state,
        [FromQuery] string code,
        [FromQuery] string? error = null,
        [FromQuery] string? error_description = null,
        CancellationToken ct = default)
    {
        if (!string.IsNullOrEmpty(error))
        {
            return BadRequest(new { error, description = error_description });
        }

        var result = await _oidcService.HandleCallbackAsync(state, code, ct).ConfigureAwait(false);

        if (!result.Success || result.User == null)
        {
            return Unauthorized(new { error = result.Error ?? "Authentication failed" });
        }

        var accessToken = _jwtTokenService.GenerateAccessToken(result.User);
        var refreshToken = await _jwtTokenService.GenerateRefreshTokenAsync(
            result.User.Id, "oidc-login", ct).ConfigureAwait(false);

        return Ok(new LoginResponse
        {
            AccessToken = accessToken,
            RefreshToken = refreshToken,
            User = AuthController.ToUserResource(result.User)
        });
    }

    // --- Admin CRUD for OIDC providers ---

    [HttpGet("admin/providers")]
    [Authorize(Roles = "Admin")]
    public async Task<ActionResult<List<OidcProviderResource>>> GetAllProviders(CancellationToken ct = default)
    {
        var providers = (await _providerRepo.AllAsync(ct).ConfigureAwait(false)).ToList();
        return Ok(providers.Select(ToResource).ToList());
    }

    [HttpGet("admin/providers/{id:int}")]
    [Authorize(Roles = "Admin")]
    public async Task<ActionResult<OidcProviderResource>> GetProvider(int id, CancellationToken ct = default)
    {
        var provider = await _providerRepo.FindAsync(id, ct).ConfigureAwait(false);
        if (provider == null) return NotFound();
        return Ok(ToResource(provider));
    }

    [HttpPost("admin/providers")]
    [Authorize(Roles = "Admin")]
    public async Task<ActionResult<OidcProviderResource>> CreateProvider(
        [FromBody][Required] CreateOidcProviderRequest request,
        CancellationToken ct = default)
    {
        if (await _providerRepo.SlugExistsAsync(request.Slug, ct).ConfigureAwait(false))
        {
            return Conflict(new { error = $"Provider with slug '{request.Slug}' already exists" });
        }

        // Validate discovery endpoint is reachable
        var provider = new OidcProvider
        {
            Name = request.Name,
            Slug = request.Slug.ToLowerInvariant(),
            IssuerUrl = request.IssuerUrl.TrimEnd('/'),
            ClientId = request.ClientId,
            ClientSecret = request.ClientSecret,
            Scopes = request.Scopes ?? "openid profile email",
            AutoProvisionUsers = request.AutoProvisionUsers ?? true,
            DefaultRole = Enum.TryParse<UserRole>(request.DefaultRole, true, out var role) ? role : UserRole.User,
            ClaimRoleMapping = request.ClaimRoleMapping ?? "{}",
            RoleClaimType = request.RoleClaimType ?? "roles",
            Enabled = request.Enabled ?? true,
            SortOrder = request.SortOrder ?? 0,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow
        };

        var created = await _providerRepo.InsertAsync(provider, ct).ConfigureAwait(false);
        return CreatedAtAction(nameof(GetProvider), new { id = created.Id }, ToResource(created));
    }

    [HttpPut("admin/providers/{id:int}")]
    [Authorize(Roles = "Admin")]
    public async Task<ActionResult<OidcProviderResource>> UpdateProvider(
        int id,
        [FromBody][Required] UpdateOidcProviderRequest request,
        CancellationToken ct = default)
    {
        var provider = await _providerRepo.FindAsync(id, ct).ConfigureAwait(false);
        if (provider == null) return NotFound();

        if (request.Name != null) provider.Name = request.Name;
        if (request.IssuerUrl != null) provider.IssuerUrl = request.IssuerUrl.TrimEnd('/');
        if (request.ClientId != null) provider.ClientId = request.ClientId;
        if (request.ClientSecret != null) provider.ClientSecret = request.ClientSecret;
        if (request.Scopes != null) provider.Scopes = request.Scopes;
        if (request.AutoProvisionUsers.HasValue) provider.AutoProvisionUsers = request.AutoProvisionUsers.Value;
        if (request.DefaultRole != null && Enum.TryParse<UserRole>(request.DefaultRole, true, out var role))
            provider.DefaultRole = role;
        if (request.ClaimRoleMapping != null) provider.ClaimRoleMapping = request.ClaimRoleMapping;
        if (request.RoleClaimType != null) provider.RoleClaimType = request.RoleClaimType;
        if (request.Enabled.HasValue) provider.Enabled = request.Enabled.Value;
        if (request.SortOrder.HasValue) provider.SortOrder = request.SortOrder.Value;

        provider.UpdatedAt = DateTime.UtcNow;
        var updated = await _providerRepo.UpdateAsync(provider, ct).ConfigureAwait(false);
        return Ok(ToResource(updated));
    }

    [HttpDelete("admin/providers/{id:int}")]
    [Authorize(Roles = "Admin")]
    public async Task<ActionResult> DeleteProvider(int id, CancellationToken ct = default)
    {
        var provider = await _providerRepo.FindAsync(id, ct).ConfigureAwait(false);
        if (provider == null) return NotFound();

        await _providerRepo.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    /// <summary>Test discovery endpoint for a provider</summary>
    [HttpPost("admin/providers/{id:int}/test")]
    [Authorize(Roles = "Admin")]
    public async Task<ActionResult<OidcDiscoveryTestResult>> TestProvider(int id, CancellationToken ct = default)
    {
        var provider = await _providerRepo.FindAsync(id, ct).ConfigureAwait(false);
        if (provider == null) return NotFound();

        try
        {
            var discovery = await _oidcService.GetDiscoveryDocumentAsync(provider, ct).ConfigureAwait(false);
            return Ok(new OidcDiscoveryTestResult
            {
                Success = true,
                AuthorizationEndpoint = discovery.AuthorizationEndpoint,
                TokenEndpoint = discovery.TokenEndpoint,
                UserinfoEndpoint = discovery.UserinfoEndpoint
            });
        }
        catch (Exception ex)
        {
            return Ok(new OidcDiscoveryTestResult { Success = false, Error = ex.Message });
        }
    }

    private static OidcProviderResource ToResource(OidcProvider p) => new()
    {
        Id = p.Id,
        Name = p.Name,
        Slug = p.Slug,
        IssuerUrl = p.IssuerUrl,
        ClientId = p.ClientId,
        HasClientSecret = !string.IsNullOrEmpty(p.ClientSecret),
        Scopes = p.Scopes,
        AutoProvisionUsers = p.AutoProvisionUsers,
        DefaultRole = p.DefaultRole.ToString(),
        ClaimRoleMapping = p.ClaimRoleMapping,
        RoleClaimType = p.RoleClaimType,
        Enabled = p.Enabled,
        SortOrder = p.SortOrder,
        CreatedAt = p.CreatedAt,
        UpdatedAt = p.UpdatedAt
    };
}

// DTOs
public class OidcProviderInfo
{
    public string Slug { get; set; } = string.Empty;
    public string Name { get; set; } = string.Empty;
    public string AuthorizeUrl { get; set; } = string.Empty;
}

public class OidcProviderResource
{
    public int Id { get; set; }
    public string Name { get; set; } = string.Empty;
    public string Slug { get; set; } = string.Empty;
    public string IssuerUrl { get; set; } = string.Empty;
    public string ClientId { get; set; } = string.Empty;
    public bool HasClientSecret { get; set; }
    public string Scopes { get; set; } = string.Empty;
    public bool AutoProvisionUsers { get; set; }
    public string DefaultRole { get; set; } = string.Empty;
    public string ClaimRoleMapping { get; set; } = string.Empty;
    public string RoleClaimType { get; set; } = string.Empty;
    public bool Enabled { get; set; }
    public int SortOrder { get; set; }
    public DateTime CreatedAt { get; set; }
    public DateTime UpdatedAt { get; set; }
}

public class CreateOidcProviderRequest
{
    public string Name { get; set; } = string.Empty;
    public string Slug { get; set; } = string.Empty;
    public string IssuerUrl { get; set; } = string.Empty;
    public string ClientId { get; set; } = string.Empty;
    public string ClientSecret { get; set; } = string.Empty;
    public string? Scopes { get; set; }
    public bool? AutoProvisionUsers { get; set; }
    public string? DefaultRole { get; set; }
    public string? ClaimRoleMapping { get; set; }
    public string? RoleClaimType { get; set; }
    public bool? Enabled { get; set; }
    public int? SortOrder { get; set; }
}

public class UpdateOidcProviderRequest
{
    public string? Name { get; set; }
    public string? IssuerUrl { get; set; }
    public string? ClientId { get; set; }
    public string? ClientSecret { get; set; }
    public string? Scopes { get; set; }
    public bool? AutoProvisionUsers { get; set; }
    public string? DefaultRole { get; set; }
    public string? ClaimRoleMapping { get; set; }
    public string? RoleClaimType { get; set; }
    public bool? Enabled { get; set; }
    public int? SortOrder { get; set; }
}

public class OidcDiscoveryTestResult
{
    public bool Success { get; set; }
    public string? AuthorizationEndpoint { get; set; }
    public string? TokenEndpoint { get; set; }
    public string? UserinfoEndpoint { get; set; }
    public string? Error { get; set; }
}

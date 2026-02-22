// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Authentication;

/// <summary>
/// Represents an OIDC/OAuth 2.0 identity provider configuration.
/// Supports any OIDC-compliant provider: Keycloak, Authentik, Authelia, Google, Azure AD.
/// </summary>
public class OidcProvider : ModelBase
{
    public string Name { get; set; } = string.Empty;
    public string Slug { get; set; } = string.Empty;
    public string IssuerUrl { get; set; } = string.Empty;
    public string ClientId { get; set; } = string.Empty;
    public string ClientSecret { get; set; } = string.Empty;
    public string Scopes { get; set; } = "openid profile email";
    public bool AutoProvisionUsers { get; set; } = true;
    public UserRole DefaultRole { get; set; } = UserRole.User;
    public string ClaimRoleMapping { get; set; } = "{}";
    public string RoleClaimType { get; set; } = "roles";
    public bool Enabled { get; set; } = true;
    public int SortOrder { get; set; }
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
    public DateTime UpdatedAt { get; set; } = DateTime.UtcNow;
}

/// <summary>
/// Cached OIDC discovery document for a provider.
/// </summary>
public class OidcDiscoveryDocument
{
    public string AuthorizationEndpoint { get; set; } = string.Empty;
    public string TokenEndpoint { get; set; } = string.Empty;
    public string UserinfoEndpoint { get; set; } = string.Empty;
    public string JwksUri { get; set; } = string.Empty;
    public string EndSessionEndpoint { get; set; } = string.Empty;
    public List<string> ScopesSupported { get; set; } = new();
    public DateTime FetchedAt { get; set; } = DateTime.UtcNow;
}

/// <summary>
/// PKCE state stored during auth code flow. Short-lived (10 min).
/// </summary>
public class OidcAuthState : ModelBase
{
    public int ProviderId { get; set; }
    public string State { get; set; } = string.Empty;
    public string CodeVerifier { get; set; } = string.Empty;
    public string RedirectUri { get; set; } = string.Empty;
    public DateTime ExpiresAt { get; set; }
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
}

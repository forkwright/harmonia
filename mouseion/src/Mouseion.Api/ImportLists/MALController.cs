// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Security.Cryptography;
using System.Text;
using System.Net.Http.Json;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.ImportLists.MAL;

namespace Mouseion.Api.ImportLists;

/// <summary>
/// MAL OAuth 2.0 with PKCE (Proof Key for Code Exchange).
/// MAL API v2 requires PKCE instead of client_secret for public clients.
/// Flow: generate code_verifier → redirect user to MAL → exchange code → token.
/// </summary>
[ApiController]
[Route("api/v3/importlist/mal")]
[Authorize]
public class MALController : ControllerBase
{
    // In-memory code verifier storage (per-session, short-lived)
    private static readonly Dictionary<string, string> PendingVerifiers = new();

    /// <summary>
    /// Start MAL OAuth flow. Returns authorization URL for user to visit.
    /// MAL uses PKCE — we generate a code_verifier and include code_challenge in the URL.
    /// </summary>
    [HttpPost("authorize")]
    public ActionResult<MALAuthResponse> StartAuthorization([FromBody] MALAuthRequest request)
    {
        if (string.IsNullOrEmpty(request.ClientId))
        {
            return BadRequest("ClientId is required");
        }

        // Generate PKCE code verifier (43-128 chars, unreserved chars)
        var codeVerifier = GenerateCodeVerifier();
        var state = Guid.NewGuid().ToString("N");

        PendingVerifiers[state] = codeVerifier;

        // MAL supports "plain" code challenge method
        var authUrl = $"https://myanimelist.net/v1/oauth2/authorize" +
                      $"?response_type=code" +
                      $"&client_id={request.ClientId}" +
                      $"&state={state}" +
                      $"&code_challenge={codeVerifier}" +
                      $"&code_challenge_method=plain";

        if (!string.IsNullOrEmpty(request.RedirectUri))
        {
            authUrl += $"&redirect_uri={Uri.EscapeDataString(request.RedirectUri)}";
        }

        return Ok(new MALAuthResponse
        {
            AuthorizationUrl = authUrl,
            State = state
        });
    }

    /// <summary>
    /// Exchange authorization code for access token.
    /// Called after user completes MAL authorization and returns with code.
    /// </summary>
    [HttpPost("token")]
    public async Task<ActionResult<MALTokenResult>> ExchangeToken(
        [FromBody] MALTokenRequest request,
        [FromServices] IHttpClientFactory httpClientFactory)
    {
        if (string.IsNullOrEmpty(request.Code) || string.IsNullOrEmpty(request.State))
        {
            return BadRequest("Code and State are required");
        }

        if (!PendingVerifiers.TryGetValue(request.State, out var codeVerifier))
        {
            return BadRequest("Invalid or expired state parameter");
        }

        PendingVerifiers.Remove(request.State);

        var client = httpClientFactory.CreateClient("MAL");
        var tokenRequest = new FormUrlEncodedContent(new Dictionary<string, string>
        {
            ["client_id"] = request.ClientId,
            ["grant_type"] = "authorization_code",
            ["code"] = request.Code,
            ["code_verifier"] = codeVerifier
        });

        if (!string.IsNullOrEmpty(request.RedirectUri))
        {
            // Re-create with redirect_uri
            tokenRequest = new FormUrlEncodedContent(new Dictionary<string, string>
            {
                ["client_id"] = request.ClientId,
                ["grant_type"] = "authorization_code",
                ["code"] = request.Code,
                ["code_verifier"] = codeVerifier,
                ["redirect_uri"] = request.RedirectUri
            });
        }

        var response = await client.PostAsync("https://myanimelist.net/v1/oauth2/token", tokenRequest);

        if (!response.IsSuccessStatusCode)
        {
            var error = await response.Content.ReadAsStringAsync();
            return StatusCode((int)response.StatusCode, $"MAL token exchange failed: {error}");
        }

        var tokenResponse = await response.Content.ReadFromJsonAsync<MALTokenResponse>();

        return Ok(new MALTokenResult
        {
            AccessToken = tokenResponse?.AccessToken ?? string.Empty,
            RefreshToken = tokenResponse?.RefreshToken ?? string.Empty,
            ExpiresIn = tokenResponse?.ExpiresIn ?? 0
        });
    }

    private static string GenerateCodeVerifier()
    {
        var bytes = new byte[64];
        using var rng = RandomNumberGenerator.Create();
        rng.GetBytes(bytes);
        return Convert.ToBase64String(bytes)
            .TrimEnd('=')
            .Replace('+', '-')
            .Replace('/', '_');
    }
}

public class MALAuthRequest
{
    public string ClientId { get; set; } = string.Empty;
    public string? RedirectUri { get; set; }
}

public class MALAuthResponse
{
    public string AuthorizationUrl { get; set; } = string.Empty;
    public string State { get; set; } = string.Empty;
}

public class MALTokenRequest
{
    public string ClientId { get; set; } = string.Empty;
    public string Code { get; set; } = string.Empty;
    public string State { get; set; } = string.Empty;
    public string? RedirectUri { get; set; }
}

public class MALTokenResult
{
    public string AccessToken { get; set; } = string.Empty;
    public string RefreshToken { get; set; } = string.Empty;
    public int ExpiresIn { get; set; }
}

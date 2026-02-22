// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.IdentityModel.Tokens.Jwt;
using System.Security.Claims;
using System.Text;
using System.Text.Encodings.Web;
using Microsoft.AspNetCore.Authentication;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using Microsoft.IdentityModel.Tokens;

namespace Mouseion.Api.Security;

public class JwtAuthenticationOptions : AuthenticationSchemeOptions
{
    public const string DefaultScheme = "Bearer";
    public string SecretKey { get; set; } = string.Empty;
    public string Issuer { get; set; } = "mouseion";
    public string Audience { get; set; } = "mouseion-clients";
}

public class JwtAuthenticationHandler : AuthenticationHandler<JwtAuthenticationOptions>
{
    // Routes that accept ?token= query parameter for browser element auth
    // (img src, audio src, video src can't set Authorization headers)
    private static readonly string[] QueryParamAllowedPrefixes =
    {
        "/api/v3/stream",
        "/api/v3/mediacover"
    };

    public JwtAuthenticationHandler(
        IOptionsMonitor<JwtAuthenticationOptions> options,
        ILoggerFactory logger,
        UrlEncoder encoder)
        : base(options, logger, encoder)
    {
    }

    protected override Task<AuthenticateResult> HandleAuthenticateAsync()
    {
        var token = ExtractToken();

        if (string.IsNullOrEmpty(token))
        {
            return Task.FromResult(AuthenticateResult.NoResult());
        }

        try
        {
            var handler = new JwtSecurityTokenHandler();
            var key = new SymmetricSecurityKey(Encoding.UTF8.GetBytes(Options.SecretKey));

            var parameters = new TokenValidationParameters
            {
                ValidateIssuer = true,
                ValidateAudience = true,
                ValidateLifetime = true,
                ValidateIssuerSigningKey = true,
                ValidIssuer = Options.Issuer,
                ValidAudience = Options.Audience,
                IssuerSigningKey = key,
                ClockSkew = TimeSpan.FromSeconds(30)
            };

            var principal = handler.ValidateToken(token, parameters, out _);
            var ticket = new AuthenticationTicket(principal, JwtAuthenticationOptions.DefaultScheme);

            return Task.FromResult(AuthenticateResult.Success(ticket));
        }
        catch (SecurityTokenExpiredException)
        {
            return Task.FromResult(AuthenticateResult.Fail("Token has expired"));
        }
        catch (Exception ex)
        {
            Logger.LogWarning(ex, "JWT validation failed");
            return Task.FromResult(AuthenticateResult.Fail("Invalid token"));
        }
    }

    private string? ExtractToken()
    {
        // 1. Authorization header (standard path)
        if (Request.Headers.TryGetValue("Authorization", out var authHeader))
        {
            var headerValue = authHeader.FirstOrDefault();
            if (!string.IsNullOrEmpty(headerValue) &&
                headerValue.StartsWith("Bearer ", StringComparison.OrdinalIgnoreCase))
            {
                var headerToken = headerValue["Bearer ".Length..].Trim();
                if (!string.IsNullOrEmpty(headerToken))
                {
                    return headerToken;
                }
            }
        }

        // 2. Query parameter fallback — only for streaming/media routes
        //    where browser elements (img, audio, video) can't set headers
        if (Request.Query.TryGetValue("token", out var queryToken))
        {
            var tokenValue = queryToken.FirstOrDefault();
            if (!string.IsNullOrEmpty(tokenValue) && IsQueryParamAllowed())
            {
                return tokenValue;
            }
        }

        return null;
    }

    private bool IsQueryParamAllowed()
    {
        var path = Request.Path.Value;
        if (string.IsNullOrEmpty(path))
        {
            return false;
        }

        foreach (var prefix in QueryParamAllowedPrefixes)
        {
            if (path.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }
        }

        return false;
    }
}

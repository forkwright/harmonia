// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.Mvc.Filters;
using Microsoft.Extensions.Configuration;

namespace Mouseion.Api.Filters;

/// <summary>
/// Action filter that validates webhook secret from X-Webhook-Secret header.
/// The secret is auto-generated and stored in the data directory on first use.
/// </summary>
public class WebhookSecretFilter : IAsyncActionFilter
{
    private readonly IConfiguration _config;
    private static string? _cachedSecret;

    public WebhookSecretFilter(IConfiguration config)
    {
        _config = config;
    }

    public async Task OnActionExecutionAsync(ActionExecutingContext context, ActionExecutionDelegate next)
    {
        var secret = GetOrCreateSecret();

        // If no secret configured/generated, allow all webhooks (backward compat)
        if (string.IsNullOrEmpty(secret))
        {
            await next();
            return;
        }

        var headerSecret = context.HttpContext.Request.Headers["X-Webhook-Secret"].FirstOrDefault();

        if (string.IsNullOrEmpty(headerSecret) || headerSecret != secret)
        {
            context.Result = new UnauthorizedObjectResult(new
            {
                error = "Invalid or missing X-Webhook-Secret header",
                hint = "Configure your media server's webhook with the secret from GET /api/v3/webhooks/secret"
            });
            return;
        }

        await next();
    }

    private string? GetOrCreateSecret()
    {
        if (_cachedSecret != null) return _cachedSecret;

        // Check config first
        _cachedSecret = _config["Webhooks:Secret"];
        if (!string.IsNullOrEmpty(_cachedSecret)) return _cachedSecret;

        // Check data directory
        var dataDir = _config["data"]
            ?? Environment.GetEnvironmentVariable("MOUSEION_DATA")
            ?? Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Mouseion");
        var secretFile = Path.Combine(dataDir, ".webhook-secret");

        if (File.Exists(secretFile))
        {
            _cachedSecret = File.ReadAllText(secretFile).Trim();
            return _cachedSecret;
        }

        // Generate and persist
        _cachedSecret = Guid.NewGuid().ToString("N");
        try
        {
            Directory.CreateDirectory(dataDir);
            File.WriteAllText(secretFile, _cachedSecret);
        }
        catch
        {
            // Can't persist — secret changes on restart but still works per-session
        }

        return _cachedSecret;
    }
}

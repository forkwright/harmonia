// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.Mvc.Filters;
using Mouseion.Core.Authentication;

namespace Mouseion.Api.Filters;

/// <summary>
/// Action filter that validates API key from query string (?apikey=xxx).
/// Used for OPDS endpoints where clients can't send Authorization headers.
/// Falls through to JWT auth if no API key provided.
/// </summary>
public class ApiKeyAuthFilter : IAsyncActionFilter
{
    private readonly Mouseion.Core.Authentication.IAuthorizationService _authService;

    public ApiKeyAuthFilter(Mouseion.Core.Authentication.IAuthorizationService authService)
    {
        _authService = authService;
    }

    public async Task OnActionExecutionAsync(ActionExecutingContext context, ActionExecutionDelegate next)
    {
        // Check for API key in query string
        var apiKey = context.HttpContext.Request.Query["apikey"].FirstOrDefault();

        if (string.IsNullOrEmpty(apiKey))
        {
            // Also check Authorization header for Bearer token — let JWT middleware handle it
            if (context.HttpContext.User.Identity?.IsAuthenticated == true)
            {
                await next();
                return;
            }

            context.Result = new UnauthorizedObjectResult(new { error = "API key required. Pass ?apikey=YOUR_KEY" });
            return;
        }

        var validKey = await _authService.ValidateApiKeyAsync(apiKey);
        if (validKey == null)
        {
            context.Result = new UnauthorizedObjectResult(new { error = "Invalid API key" });
            return;
        }

        // Check scope — OPDS needs at least "read" scope
        if (!string.IsNullOrEmpty(validKey.Scopes) && !validKey.Scopes.Contains("read"))
        {
            context.Result = new ObjectResult(new { error = "API key lacks 'read' scope" }) { StatusCode = 403 };
            return;
        }

        await next();
    }
}

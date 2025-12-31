// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;

namespace Mouseion.Api.Security
{
    public class SecurityHeadersMiddleware
    {
        private readonly RequestDelegate _next;

        public SecurityHeadersMiddleware(RequestDelegate next)
        {
            _next = next;
        }

        // Note: SonarCloud suggests making this static, but middleware pattern requires instance method
#pragma warning disable S2325 // Methods and properties that don't access instance data should be static
        public async Task InvokeAsync(HttpContext context)
#pragma warning restore S2325
        {
            // Strict-Transport-Security (HSTS)
            context.Response.Headers.Append("Strict-Transport-Security", "max-age=31536000; includeSubDomains");

            // X-Content-Type-Options
            context.Response.Headers.Append("X-Content-Type-Options", "nosniff");

            // X-Frame-Options
            context.Response.Headers.Append("X-Frame-Options", "DENY");

            // Content-Security-Policy
            context.Response.Headers.Append("Content-Security-Policy",
                "default-src 'self'; " +
                "script-src 'self'; " +
                "style-src 'self' 'unsafe-inline'; " +
                "img-src 'self' data:; " +
                "font-src 'self'; " +
                "connect-src 'self'; " +
                "frame-ancestors 'none'");

            // X-Permitted-Cross-Domain-Policies
            context.Response.Headers.Append("X-Permitted-Cross-Domain-Policies", "none");

            // Referrer-Policy
            context.Response.Headers.Append("Referrer-Policy", "strict-origin-when-cross-origin");

            // Permissions-Policy
            context.Response.Headers.Append("Permissions-Policy",
                "camera=(), microphone=(), geolocation=(), payment=()");

            await _next(context);
        }
    }

    public static class SecurityHeadersMiddlewareExtensions
    {
        public static IApplicationBuilder UseSecurityHeaders(this IApplicationBuilder builder)
        {
            return builder.UseMiddleware<SecurityHeadersMiddleware>();
        }
    }
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Diagnostics;
using Microsoft.AspNetCore.Http;
using Mouseion.Common.Instrumentation;

namespace Mouseion.Api.Middleware
{
    /// <summary>
    /// Middleware that captures detailed telemetry for all HTTP requests.
    /// Complements the built-in OpenTelemetry ASP.NET Core instrumentation with
    /// custom Mouseion-specific metrics and span attributes.
    /// </summary>
    public class TelemetryMiddleware
    {
        private readonly RequestDelegate _next;

        public TelemetryMiddleware(RequestDelegate next)
        {
            _next = next;
        }

        public async Task InvokeAsync(HttpContext context)
        {
            // Skip telemetry for health and metrics endpoints
            var path = context.Request.Path.Value ?? string.Empty;
            if (path.StartsWith("/health", StringComparison.OrdinalIgnoreCase) ||
                path.StartsWith("/metrics", StringComparison.OrdinalIgnoreCase))
            {
                await _next(context);
                return;
            }

            var stopwatch = Stopwatch.StartNew();
            var endpoint = GetNormalizedEndpoint(context);
            var method = context.Request.Method;

            // Add custom attributes to current activity
            var activity = Activity.Current;
            activity?.SetTag("mouseion.endpoint", endpoint);
            activity?.SetTag("mouseion.api_version", "v3");

            // Add user info if authenticated
            if (context.User.Identity?.IsAuthenticated == true)
            {
                activity?.SetTag("enduser.id", context.User.Identity.Name);
            }

            try
            {
                await _next(context);

                stopwatch.Stop();
                var statusCode = context.Response.StatusCode;

                // Record metrics
                MouseionMetrics.RecordApiRequest(endpoint, method, statusCode, stopwatch.Elapsed.TotalMilliseconds);

                // Set success/error status on activity
                if (statusCode >= 400)
                {
                    activity?.SetStatus(ActivityStatusCode.Error, $"HTTP {statusCode}");
                    MouseionMetrics.RecordApiError(endpoint, GetErrorCategory(statusCode));
                }
                else
                {
                    activity?.SetStatus(ActivityStatusCode.Ok);
                }
            }
            catch (Exception ex)
            {
                stopwatch.Stop();

                // Record error metrics
                MouseionMetrics.RecordApiRequest(endpoint, method, 500, stopwatch.Elapsed.TotalMilliseconds);
                MouseionMetrics.RecordApiError(endpoint, "exception", ex.GetType().Name);

                // Set error on activity
                MouseionMetrics.SetActivityError(activity, ex);

                throw;
            }
        }

        private static string GetNormalizedEndpoint(HttpContext context)
        {
            // Try to get the route template for consistent metric labels
            var endpoint = context.GetEndpoint();
            if (endpoint is Microsoft.AspNetCore.Routing.RouteEndpoint routeEndpoint)
            {
                return routeEndpoint.RoutePattern.RawText ?? context.Request.Path.Value ?? "/";
            }

            // Fallback to path, but normalize numeric IDs to {id} to avoid cardinality explosion
            var path = context.Request.Path.Value ?? "/";
            return NormalizePath(path);
        }

        private static string NormalizePath(string path)
        {
            // Replace numeric segments with {id} to reduce metric cardinality
            // e.g., /api/v3/movies/123 -> /api/v3/movies/{id}
            var segments = path.Split('/');
            for (int i = 0; i < segments.Length; i++)
            {
                if (int.TryParse(segments[i], out _) || Guid.TryParse(segments[i], out _))
                {
                    segments[i] = "{id}";
                }
            }
            return string.Join("/", segments);
        }

        private static string GetErrorCategory(int statusCode) => statusCode switch
        {
            400 => "bad_request",
            401 => "unauthorized",
            403 => "forbidden",
            404 => "not_found",
            409 => "conflict",
            422 => "validation_error",
            429 => "rate_limited",
            >= 500 and < 600 => "server_error",
            _ => "client_error"
        };
    }

    /// <summary>
    /// Extension methods for adding telemetry middleware to the pipeline.
    /// </summary>
    public static class TelemetryMiddlewareExtensions
    {
        /// <summary>
        /// Adds the Mouseion telemetry middleware to capture custom metrics and span attributes.
        /// Should be added early in the pipeline, after exception handling.
        /// </summary>
        public static IApplicationBuilder UseMouseionTelemetry(this IApplicationBuilder app)
        {
            return app.UseMiddleware<TelemetryMiddleware>();
        }
    }
}

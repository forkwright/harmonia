// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Diagnostics;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Mouseion.Common.EnvironmentInfo;
using OpenTelemetry.Exporter;
using OpenTelemetry.Metrics;
using OpenTelemetry.Resources;
using OpenTelemetry.Trace;

namespace Mouseion.Common.Instrumentation
{
    /// <summary>
    /// Configuration options for OpenTelemetry telemetry.
    /// </summary>
    public class TelemetryOptions
    {
        public const string SectionName = "Telemetry";

        /// <summary>
        /// Enable or disable telemetry collection. Default: true.
        /// </summary>
        public bool Enabled { get; set; } = true;

        /// <summary>
        /// Enable console exporter for debugging. Default: false in production.
        /// </summary>
        public bool EnableConsoleExporter { get; set; }

        /// <summary>
        /// Enable Prometheus metrics endpoint at /metrics. Default: true.
        /// </summary>
        public bool EnablePrometheus { get; set; } = true;

        /// <summary>
        /// OTLP endpoint for traces and metrics (e.g., http://localhost:4317 for Jaeger).
        /// Leave empty to disable OTLP export.
        /// </summary>
        public string? OtlpEndpoint { get; set; }

        /// <summary>
        /// Service instance ID for resource attributes.
        /// </summary>
        public string? ServiceInstanceId { get; set; }

        /// <summary>
        /// Additional resource attributes as key=value pairs.
        /// </summary>
        public Dictionary<string, string> ResourceAttributes { get; set; } = new();
    }

    public static class OpenTelemetryConfiguration
    {
        /// <summary>
        /// ActivitySource for creating distributed tracing spans.
        /// </summary>
        public static readonly ActivitySource ActivitySource = new(BuildInfo.AppName, BuildInfo.Version.ToString());

        /// <summary>
        /// Configures OpenTelemetry with tracing, metrics, and exporters.
        /// </summary>
        public static IServiceCollection AddMouseionTelemetry(this IServiceCollection services, IConfiguration? configuration = null)
        {
            var options = new TelemetryOptions();
            configuration?.GetSection(TelemetryOptions.SectionName).Bind(options);

            if (!options.Enabled)
            {
                return services;
            }

            var resourceBuilder = ResourceBuilder.CreateDefault()
                .AddService(
                    serviceName: BuildInfo.AppName,
                    serviceVersion: BuildInfo.Version.ToString(),
                    serviceInstanceId: options.ServiceInstanceId ?? Environment.MachineName)
                .AddAttributes(new[]
                {
                    new KeyValuePair<string, object>("deployment.environment",
                        Environment.GetEnvironmentVariable("ASPNETCORE_ENVIRONMENT") ?? "Production"),
                    new KeyValuePair<string, object>("host.name", Environment.MachineName),
                    new KeyValuePair<string, object>("os.type", Environment.OSVersion.Platform.ToString()),
                    new KeyValuePair<string, object>("runtime.version", Environment.Version.ToString())
                });

            // Add custom resource attributes
            if (options.ResourceAttributes.Count > 0)
            {
                resourceBuilder.AddAttributes(
                    options.ResourceAttributes.Select(kv =>
                        new KeyValuePair<string, object>(kv.Key, kv.Value)));
            }

            var otelBuilder = services.AddOpenTelemetry();

            // Configure tracing
            otelBuilder.WithTracing(builder =>
            {
                builder
                    .SetResourceBuilder(resourceBuilder)
                    .AddSource(BuildInfo.AppName)
                    .AddSource(MouseionMetrics.SourceName)
                    .AddHttpClientInstrumentation(opts =>
                    {
                        opts.RecordException = true;
                        opts.EnrichWithHttpRequestMessage = (activity, request) =>
                        {
                            activity?.SetTag("http.request.host", request.RequestUri?.Host);
                        };
                        opts.EnrichWithHttpResponseMessage = (activity, response) =>
                        {
                            activity?.SetTag("http.response.content_length",
                                response.Content.Headers.ContentLength);
                        };
                    })
                    .AddAspNetCoreInstrumentation(opts =>
                    {
                        opts.RecordException = true;
                        opts.EnrichWithHttpRequest = (activity, request) =>
                        {
                            activity?.SetTag("http.client_ip", request.HttpContext.Connection.RemoteIpAddress?.ToString());
                            activity?.SetTag("http.request_content_length", request.ContentLength);
                        };
                        opts.EnrichWithHttpResponse = (activity, response) =>
                        {
                            activity?.SetTag("http.response_content_length", response.ContentLength);
                        };
                        opts.Filter = httpContext =>
                        {
                            // Skip health check and metrics endpoints from tracing
                            var path = httpContext.Request.Path.Value ?? string.Empty;
                            return !path.StartsWith("/health", StringComparison.OrdinalIgnoreCase)
                                && !path.StartsWith("/metrics", StringComparison.OrdinalIgnoreCase);
                        };
                    });

                if (options.EnableConsoleExporter)
                {
                    builder.AddConsoleExporter();
                }

                if (!string.IsNullOrEmpty(options.OtlpEndpoint))
                {
                    builder.AddOtlpExporter(opts =>
                    {
                        opts.Endpoint = new Uri(options.OtlpEndpoint);
                        opts.Protocol = OtlpExportProtocol.Grpc;
                    });
                }
            });

            // Configure metrics
            otelBuilder.WithMetrics(builder =>
            {
                builder
                    .SetResourceBuilder(resourceBuilder)
                    .AddMeter(BuildInfo.AppName)
                    .AddMeter(MouseionMetrics.MeterName)
                    .AddRuntimeInstrumentation()
                    .AddHttpClientInstrumentation()
                    .AddAspNetCoreInstrumentation();

                if (options.EnableConsoleExporter)
                {
                    builder.AddConsoleExporter();
                }

                if (options.EnablePrometheus)
                {
                    builder.AddPrometheusExporter();
                }

                if (!string.IsNullOrEmpty(options.OtlpEndpoint))
                {
                    builder.AddOtlpExporter(opts =>
                    {
                        opts.Endpoint = new Uri(options.OtlpEndpoint);
                        opts.Protocol = OtlpExportProtocol.Grpc;
                    });
                }
            });

            return services;
        }

        /// <summary>
        /// Starts a new activity (span) for distributed tracing.
        /// </summary>
        public static Activity? StartActivity(string name, ActivityKind kind = ActivityKind.Internal)
        {
            return ActivitySource.StartActivity(name, kind);
        }

        /// <summary>
        /// Starts a new activity with a specific parent context.
        /// </summary>
        public static Activity? StartActivity(string name, ActivityKind kind, ActivityContext parentContext)
        {
            return ActivitySource.StartActivity(name, kind, parentContext);
        }

        /// <summary>
        /// Starts a new activity with tags.
        /// </summary>
        public static Activity? StartActivity(string name, ActivityKind kind, IEnumerable<KeyValuePair<string, object?>> tags)
        {
            return ActivitySource.StartActivity(name, kind, default(ActivityContext), tags);
        }
    }
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Diagnostics;
using Microsoft.Extensions.DependencyInjection;
using Mouseion.Common.EnvironmentInfo;
using OpenTelemetry.Metrics;
using OpenTelemetry.Resources;
using OpenTelemetry.Trace;

namespace Mouseion.Common.Instrumentation
{
    public static class OpenTelemetryConfiguration
    {
        public static readonly ActivitySource ActivitySource = new ActivitySource(BuildInfo.AppName, BuildInfo.Version.ToString());

        public static IServiceCollection AddMouseionTelemetry(this IServiceCollection services)
        {
            var resourceBuilder = ResourceBuilder.CreateDefault()
                .AddService(
                    serviceName: BuildInfo.AppName,
                    serviceVersion: BuildInfo.Version.ToString());

            services.AddOpenTelemetry()
                .WithTracing(builder =>
                {
                    builder
                        .SetResourceBuilder(resourceBuilder)
                        .AddSource(BuildInfo.AppName)
                        .AddHttpClientInstrumentation()
                        .AddConsoleExporter();
                })
                .WithMetrics(builder =>
                {
                    builder
                        .SetResourceBuilder(resourceBuilder)
                        .AddRuntimeInstrumentation()
                        .AddHttpClientInstrumentation()
                        .AddConsoleExporter();
                });

            return services;
        }

        public static Activity? StartActivity(string name, ActivityKind kind = ActivityKind.Internal)
        {
            return ActivitySource.StartActivity(name, kind);
        }

        public static Activity? StartActivity(string name, ActivityKind kind, ActivityContext parentContext)
        {
            return ActivitySource.StartActivity(name, kind, parentContext);
        }
    }
}

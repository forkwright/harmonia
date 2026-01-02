// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Diagnostics.Metrics;
using Mouseion.Common.EnvironmentInfo;

namespace Mouseion.Common.Instrumentation
{
    public static class DiagnosticsContext
    {
        private static readonly Meter Meter = new Meter(BuildInfo.AppName, BuildInfo.Version.ToString());

        private static readonly Counter<long> RequestCounter = Meter.CreateCounter<long>(
            "mouseion.requests.total",
            description: "Total number of requests processed");

        private static readonly Counter<long> ErrorCounter = Meter.CreateCounter<long>(
            "mouseion.errors.total",
            description: "Total number of errors");

        private static readonly Histogram<double> RequestDuration = Meter.CreateHistogram<double>(
            "mouseion.request.duration",
            unit: "ms",
            description: "Request duration in milliseconds");

        public static void RecordRequest(string endpoint, string method)
        {
            RequestCounter.Add(1, new KeyValuePair<string, object?>("endpoint", endpoint),
                                 new KeyValuePair<string, object?>("method", method));
        }

        public static void RecordError(string errorType, string? source = null)
        {
            var tags = new List<KeyValuePair<string, object?>>
            {
                new KeyValuePair<string, object?>("error_type", errorType)
            };

            if (source != null)
            {
                tags.Add(new KeyValuePair<string, object?>("source", source));
            }

            ErrorCounter.Add(1, tags.ToArray());
        }

        public static void RecordRequestDuration(string endpoint, double durationMs)
        {
            RequestDuration.Record(durationMs, new KeyValuePair<string, object?>("endpoint", endpoint));
        }

        public static IDisposable? BeginScope(string name)
        {
            return OpenTelemetryConfiguration.StartActivity(name);
        }

        public static void AddTag(Activity? activity, string key, object? value)
        {
            activity?.SetTag(key, value);
        }

        public static void AddEvent(Activity? activity, string name, params KeyValuePair<string, object?>[] tags)
        {
            var activityTags = new ActivityTagsCollection();
            foreach (var tag in tags)
            {
                activityTags.Add(tag.Key, tag.Value);
            }

            activity?.AddEvent(new ActivityEvent(name, tags: activityTags));
        }
    }
}

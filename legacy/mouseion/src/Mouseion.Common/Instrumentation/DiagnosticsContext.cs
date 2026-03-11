// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Diagnostics;

namespace Mouseion.Common.Instrumentation
{
    /// <summary>
    /// Legacy diagnostics context providing backward compatibility with existing code.
    /// New code should use <see cref="MouseionMetrics"/> directly.
    /// </summary>
    public static class DiagnosticsContext
    {
        /// <summary>
        /// Records an API request metric.
        /// </summary>
        public static void RecordRequest(string endpoint, string method)
        {
            MouseionMetrics.RecordApiRequest(endpoint, method, 200, 0);
        }

        /// <summary>
        /// Records an error metric.
        /// </summary>
        public static void RecordError(string errorType, string? source = null)
        {
            MouseionMetrics.RecordApiError(source ?? "unknown", errorType);
        }

        /// <summary>
        /// Records request duration metric.
        /// </summary>
        public static void RecordRequestDuration(string endpoint, double durationMs)
        {
            MouseionMetrics.RecordApiRequest(endpoint, "GET", 200, durationMs);
        }

        /// <summary>
        /// Begins a new diagnostic scope (activity span).
        /// </summary>
        public static IDisposable? BeginScope(string name)
        {
            return OpenTelemetryConfiguration.StartActivity(name);
        }

        /// <summary>
        /// Adds a tag to an activity.
        /// </summary>
        public static void AddTag(Activity? activity, string key, object? value)
        {
            activity?.SetTag(key, value);
        }

        /// <summary>
        /// Adds an event to an activity.
        /// </summary>
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

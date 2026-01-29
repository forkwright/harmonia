// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Diagnostics;
using System.Diagnostics.Metrics;
using Mouseion.Common.EnvironmentInfo;

namespace Mouseion.Common.Instrumentation
{
    /// <summary>
    /// Centralized metrics collection for Mouseion.
    /// Provides counters, histograms, and gauges for observability.
    /// </summary>
    public static class MouseionMetrics
    {
        public const string MeterName = "Mouseion.Metrics";
        public const string SourceName = "Mouseion.Tracing";

        private static readonly Meter Meter = new(MeterName, BuildInfo.Version.ToString());
        private static readonly ActivitySource ActivitySource = new(SourceName, BuildInfo.Version.ToString());

        #region API Metrics

        private static readonly Counter<long> ApiRequestsTotal = Meter.CreateCounter<long>(
            "mouseion_api_requests_total",
            unit: "{requests}",
            description: "Total number of API requests");

        private static readonly Counter<long> ApiErrorsTotal = Meter.CreateCounter<long>(
            "mouseion_api_errors_total",
            unit: "{errors}",
            description: "Total number of API errors by type");

        private static readonly Histogram<double> ApiRequestDuration = Meter.CreateHistogram<double>(
            "mouseion_api_request_duration_milliseconds",
            unit: "ms",
            description: "API request duration in milliseconds");

        public static void RecordApiRequest(string endpoint, string method, int statusCode, double durationMs)
        {
            var tags = new TagList
            {
                { "endpoint", endpoint },
                { "method", method },
                { "status_code", statusCode.ToString() }
            };
            ApiRequestsTotal.Add(1, tags);
            ApiRequestDuration.Record(durationMs, tags);
        }

        public static void RecordApiError(string endpoint, string errorType, string? exceptionType = null)
        {
            var tags = new TagList
            {
                { "endpoint", endpoint },
                { "error_type", errorType }
            };
            if (exceptionType != null)
            {
                tags.Add("exception_type", exceptionType);
            }
            ApiErrorsTotal.Add(1, tags);
        }

        #endregion

        #region Media Operations Metrics

        private static readonly Counter<long> MediaImportsTotal = Meter.CreateCounter<long>(
            "mouseion_media_imports_total",
            unit: "{imports}",
            description: "Total number of media imports by type and status");

        private static readonly Counter<long> MediaDownloadsTotal = Meter.CreateCounter<long>(
            "mouseion_media_downloads_total",
            unit: "{downloads}",
            description: "Total number of media downloads by type and status");

        private static readonly Counter<long> TranscodesTotal = Meter.CreateCounter<long>(
            "mouseion_transcodes_total",
            unit: "{transcodes}",
            description: "Total number of transcode operations");

        private static readonly Histogram<double> ImportDuration = Meter.CreateHistogram<double>(
            "mouseion_import_duration_milliseconds",
            unit: "ms",
            description: "Media import duration in milliseconds");

        private static readonly Histogram<long> ImportFileSize = Meter.CreateHistogram<long>(
            "mouseion_import_file_size_bytes",
            unit: "By",
            description: "Size of imported files in bytes");

        public static void RecordMediaImport(string mediaType, string status, double durationMs, long? fileSizeBytes = null)
        {
            var tags = new TagList
            {
                { "media_type", mediaType },
                { "status", status }
            };
            MediaImportsTotal.Add(1, tags);
            ImportDuration.Record(durationMs, tags);
            if (fileSizeBytes.HasValue)
            {
                ImportFileSize.Record(fileSizeBytes.Value, tags);
            }
        }

        public static void RecordMediaDownload(string mediaType, string status, string? client = null)
        {
            var tags = new TagList
            {
                { "media_type", mediaType },
                { "status", status }
            };
            if (client != null)
            {
                tags.Add("client", client);
            }
            MediaDownloadsTotal.Add(1, tags);
        }

        public static void RecordTranscode(string mediaType, string format, string status)
        {
            TranscodesTotal.Add(1, new TagList
            {
                { "media_type", mediaType },
                { "format", format },
                { "status", status }
            });
        }

        #endregion

        #region Metadata Provider Metrics

        private static readonly Counter<long> MetadataRequestsTotal = Meter.CreateCounter<long>(
            "mouseion_metadata_requests_total",
            unit: "{requests}",
            description: "Total number of metadata provider requests");

        private static readonly Counter<long> MetadataErrorsTotal = Meter.CreateCounter<long>(
            "mouseion_metadata_errors_total",
            unit: "{errors}",
            description: "Total number of metadata provider errors");

        private static readonly Histogram<double> MetadataRequestDuration = Meter.CreateHistogram<double>(
            "mouseion_metadata_request_duration_milliseconds",
            unit: "ms",
            description: "Metadata provider request duration in milliseconds");

        public static void RecordMetadataRequest(string provider, string operation, bool success, double durationMs)
        {
            var tags = new TagList
            {
                { "provider", provider },
                { "operation", operation },
                { "success", success.ToString().ToLowerInvariant() }
            };
            MetadataRequestsTotal.Add(1, tags);
            MetadataRequestDuration.Record(durationMs, tags);
            if (!success)
            {
                MetadataErrorsTotal.Add(1, tags);
            }
        }

        #endregion

        #region Download Client Metrics

        private static readonly Counter<long> DownloadClientRequestsTotal = Meter.CreateCounter<long>(
            "mouseion_download_client_requests_total",
            unit: "{requests}",
            description: "Total number of download client API calls");

        private static readonly Histogram<double> DownloadClientResponseTime = Meter.CreateHistogram<double>(
            "mouseion_download_client_response_time_milliseconds",
            unit: "ms",
            description: "Download client API response time in milliseconds");

        public static void RecordDownloadClientRequest(string client, string operation, bool success, double durationMs)
        {
            var tags = new TagList
            {
                { "client", client },
                { "operation", operation },
                { "success", success.ToString().ToLowerInvariant() }
            };
            DownloadClientRequestsTotal.Add(1, tags);
            DownloadClientResponseTime.Record(durationMs, tags);
        }

        #endregion

        #region Database Metrics

        private static readonly Counter<long> DatabaseQueriesTotal = Meter.CreateCounter<long>(
            "mouseion_database_queries_total",
            unit: "{queries}",
            description: "Total number of database queries");

        private static readonly Histogram<double> DatabaseQueryDuration = Meter.CreateHistogram<double>(
            "mouseion_database_query_duration_milliseconds",
            unit: "ms",
            description: "Database query duration in milliseconds");

        private static readonly Counter<long> DatabaseConnectionsTotal = Meter.CreateCounter<long>(
            "mouseion_database_connections_total",
            unit: "{connections}",
            description: "Total number of database connections opened");

        public static void RecordDatabaseQuery(string operation, string table, double durationMs, int? rowCount = null)
        {
            var tags = new TagList
            {
                { "operation", operation },
                { "table", table }
            };
            DatabaseQueriesTotal.Add(1, tags);
            DatabaseQueryDuration.Record(durationMs, tags);
        }

        public static void RecordDatabaseConnection(string database)
        {
            DatabaseConnectionsTotal.Add(1, new TagList { { "database", database } });
        }

        #endregion

        #region Cache Metrics

        private static readonly Counter<long> CacheHitsTotal = Meter.CreateCounter<long>(
            "mouseion_cache_hits_total",
            unit: "{hits}",
            description: "Total number of cache hits");

        private static readonly Counter<long> CacheMissesTotal = Meter.CreateCounter<long>(
            "mouseion_cache_misses_total",
            unit: "{misses}",
            description: "Total number of cache misses");

        private static readonly Counter<long> CacheEvictionsTotal = Meter.CreateCounter<long>(
            "mouseion_cache_evictions_total",
            unit: "{evictions}",
            description: "Total number of cache evictions");

        public static void RecordCacheHit(string cacheName, string? key = null)
        {
            CacheHitsTotal.Add(1, new TagList { { "cache", cacheName } });
        }

        public static void RecordCacheMiss(string cacheName, string? key = null)
        {
            CacheMissesTotal.Add(1, new TagList { { "cache", cacheName } });
        }

        public static void RecordCacheEviction(string cacheName, string? reason = null)
        {
            var tags = new TagList { { "cache", cacheName } };
            if (reason != null)
            {
                tags.Add("reason", reason);
            }
            CacheEvictionsTotal.Add(1, tags);
        }

        #endregion

        #region Background Job Metrics

        private static readonly Counter<long> JobsTotal = Meter.CreateCounter<long>(
            "mouseion_jobs_total",
            unit: "{jobs}",
            description: "Total number of background jobs executed");

        private static readonly Counter<long> JobFailuresTotal = Meter.CreateCounter<long>(
            "mouseion_job_failures_total",
            unit: "{failures}",
            description: "Total number of failed background jobs");

        private static readonly Histogram<double> JobDuration = Meter.CreateHistogram<double>(
            "mouseion_job_duration_milliseconds",
            unit: "ms",
            description: "Background job execution duration in milliseconds");

        private static readonly ObservableGauge<int> ActiveJobs = Meter.CreateObservableGauge<int>(
            "mouseion_active_jobs",
            () => _activeJobCount,
            unit: "{jobs}",
            description: "Number of currently running background jobs");

        private static int _activeJobCount;

        public static void RecordJobStarted(string jobName, string jobType)
        {
            Interlocked.Increment(ref _activeJobCount);
        }

        public static void RecordJobCompleted(string jobName, string jobType, bool success, double durationMs)
        {
            Interlocked.Decrement(ref _activeJobCount);

            var tags = new TagList
            {
                { "job_name", jobName },
                { "job_type", jobType },
                { "success", success.ToString().ToLowerInvariant() }
            };
            JobsTotal.Add(1, tags);
            JobDuration.Record(durationMs, tags);

            if (!success)
            {
                JobFailuresTotal.Add(1, tags);
            }
        }

        #endregion

        #region Library Metrics

        private static long _movieCount;
        private static long _bookCount;
        private static long _audiobookCount;
        private static long _albumCount;
        private static long _trackCount;
        private static long _tvShowCount;
        private static long _podcastCount;

        private static readonly ObservableGauge<long> LibraryItemsGauge = Meter.CreateObservableGauge<long>(
            "mouseion_library_items",
            observeValues: () => new[]
            {
                new Measurement<long>(_movieCount, new TagList { { "type", "movies" } }),
                new Measurement<long>(_bookCount, new TagList { { "type", "books" } }),
                new Measurement<long>(_audiobookCount, new TagList { { "type", "audiobooks" } }),
                new Measurement<long>(_albumCount, new TagList { { "type", "albums" } }),
                new Measurement<long>(_trackCount, new TagList { { "type", "tracks" } }),
                new Measurement<long>(_tvShowCount, new TagList { { "type", "tv_shows" } }),
                new Measurement<long>(_podcastCount, new TagList { { "type", "podcasts" } })
            },
            unit: "{items}",
            description: "Number of items in the library by type");

        public static void SetLibraryCounts(
            long? movies = null,
            long? books = null,
            long? audiobooks = null,
            long? albums = null,
            long? tracks = null,
            long? tvShows = null,
            long? podcasts = null)
        {
            if (movies.HasValue) Interlocked.Exchange(ref _movieCount, movies.Value);
            if (books.HasValue) Interlocked.Exchange(ref _bookCount, books.Value);
            if (audiobooks.HasValue) Interlocked.Exchange(ref _audiobookCount, audiobooks.Value);
            if (albums.HasValue) Interlocked.Exchange(ref _albumCount, albums.Value);
            if (tracks.HasValue) Interlocked.Exchange(ref _trackCount, tracks.Value);
            if (tvShows.HasValue) Interlocked.Exchange(ref _tvShowCount, tvShows.Value);
            if (podcasts.HasValue) Interlocked.Exchange(ref _podcastCount, podcasts.Value);
        }

        #endregion

        #region Indexer Metrics

        private static readonly Counter<long> IndexerSearchesTotal = Meter.CreateCounter<long>(
            "mouseion_indexer_searches_total",
            unit: "{searches}",
            description: "Total number of indexer searches");

        private static readonly Histogram<double> IndexerSearchDuration = Meter.CreateHistogram<double>(
            "mouseion_indexer_search_duration_milliseconds",
            unit: "ms",
            description: "Indexer search duration in milliseconds");

        private static readonly Counter<long> IndexerResultsTotal = Meter.CreateCounter<long>(
            "mouseion_indexer_results_total",
            unit: "{results}",
            description: "Total number of results returned by indexers");

        public static void RecordIndexerSearch(string indexer, string searchType, bool success, double durationMs, int resultCount)
        {
            var tags = new TagList
            {
                { "indexer", indexer },
                { "search_type", searchType },
                { "success", success.ToString().ToLowerInvariant() }
            };
            IndexerSearchesTotal.Add(1, tags);
            IndexerSearchDuration.Record(durationMs, tags);
            IndexerResultsTotal.Add(resultCount, tags);
        }

        #endregion

        #region Tracing Helpers

        /// <summary>
        /// Starts a new activity span for tracing.
        /// </summary>
        public static Activity? StartActivity(string name, ActivityKind kind = ActivityKind.Internal)
        {
            return ActivitySource.StartActivity(name, kind);
        }

        /// <summary>
        /// Starts a database operation span.
        /// </summary>
        public static Activity? StartDatabaseActivity(string operation, string table)
        {
            var activity = ActivitySource.StartActivity($"db.{operation}", ActivityKind.Client);
            activity?.SetTag("db.system", "sqlite");
            activity?.SetTag("db.operation", operation);
            activity?.SetTag("db.sql.table", table);
            return activity;
        }

        /// <summary>
        /// Starts an HTTP client span for external API calls.
        /// </summary>
        public static Activity? StartHttpActivity(string method, string url, string? provider = null)
        {
            var activity = ActivitySource.StartActivity($"http.{method.ToLowerInvariant()}", ActivityKind.Client);
            activity?.SetTag("http.method", method);
            activity?.SetTag("http.url", url);
            if (provider != null)
            {
                activity?.SetTag("mouseion.provider", provider);
            }
            return activity;
        }

        /// <summary>
        /// Starts a background job span.
        /// </summary>
        public static Activity? StartJobActivity(string jobName, string jobType)
        {
            var activity = ActivitySource.StartActivity($"job.{jobName}", ActivityKind.Internal);
            activity?.SetTag("job.name", jobName);
            activity?.SetTag("job.type", jobType);
            return activity;
        }

        /// <summary>
        /// Sets error information on an activity.
        /// </summary>
        public static void SetActivityError(Activity? activity, Exception ex)
        {
            if (activity == null) return;
            activity.SetStatus(ActivityStatusCode.Error, ex.Message);
            activity.SetTag("error.type", ex.GetType().Name);
            activity.SetTag("error.message", ex.Message);
            activity.AddEvent(new ActivityEvent("exception", tags: new ActivityTagsCollection
            {
                { "exception.type", ex.GetType().FullName },
                { "exception.message", ex.Message },
                { "exception.stacktrace", ex.StackTrace }
            }));
        }

        #endregion
    }
}

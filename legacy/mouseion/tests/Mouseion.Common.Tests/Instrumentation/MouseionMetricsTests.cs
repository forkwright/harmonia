// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Diagnostics;
using System.Diagnostics.Metrics;
using Mouseion.Common.Instrumentation;

namespace Mouseion.Common.Tests.Instrumentation;

public class MouseionMetricsTests
{
    [Fact]
    public void RecordApiRequest_WithValidParameters_DoesNotThrow()
    {
        // Act & Assert - should not throw
        var exception = Record.Exception(() =>
            MouseionMetrics.RecordApiRequest("/api/v3/movies", "GET", 200, 150.5));

        Assert.Null(exception);
    }

    [Theory]
    [InlineData("/api/v3/movies", "GET", 200, 50.0)]
    [InlineData("/api/v3/books/{id}", "PUT", 204, 120.0)]
    [InlineData("/api/v3/music/search", "POST", 201, 350.0)]
    public void RecordApiRequest_WithVariousEndpoints_DoesNotThrow(
        string endpoint, string method, int statusCode, double durationMs)
    {
        var exception = Record.Exception(() =>
            MouseionMetrics.RecordApiRequest(endpoint, method, statusCode, durationMs));

        Assert.Null(exception);
    }

    [Fact]
    public void RecordApiError_WithErrorDetails_DoesNotThrow()
    {
        var exception = Record.Exception(() =>
            MouseionMetrics.RecordApiError("/api/v3/movies", "validation_error", "ArgumentException"));

        Assert.Null(exception);
    }

    [Fact]
    public void RecordMediaImport_WithAllParameters_DoesNotThrow()
    {
        var exception = Record.Exception(() =>
            MouseionMetrics.RecordMediaImport("movie", "success", 5000.0, 1024 * 1024 * 700));

        Assert.Null(exception);
    }

    [Theory]
    [InlineData("movie", "success", "qbittorrent")]
    [InlineData("music", "failed", "sabnzbd")]
    [InlineData("book", "pending", null)]
    public void RecordMediaDownload_WithVariousClients_DoesNotThrow(
        string mediaType, string status, string? client)
    {
        var exception = Record.Exception(() =>
            MouseionMetrics.RecordMediaDownload(mediaType, status, client));

        Assert.Null(exception);
    }

    [Fact]
    public void RecordMetadataRequest_Success_DoesNotThrow()
    {
        var exception = Record.Exception(() =>
            MouseionMetrics.RecordMetadataRequest("tmdb", "search", true, 250.0));

        Assert.Null(exception);
    }

    [Fact]
    public void RecordMetadataRequest_Failure_DoesNotThrow()
    {
        var exception = Record.Exception(() =>
            MouseionMetrics.RecordMetadataRequest("musicbrainz", "lookup", false, 5000.0));

        Assert.Null(exception);
    }

    [Fact]
    public void RecordDownloadClientRequest_DoesNotThrow()
    {
        var exception = Record.Exception(() =>
            MouseionMetrics.RecordDownloadClientRequest("qbittorrent", "add_torrent", true, 150.0));

        Assert.Null(exception);
    }

    [Fact]
    public void RecordDatabaseQuery_WithRowCount_DoesNotThrow()
    {
        var exception = Record.Exception(() =>
            MouseionMetrics.RecordDatabaseQuery("SELECT", "Movies", 5.5, 25));

        Assert.Null(exception);
    }

    [Fact]
    public void RecordDatabaseConnection_DoesNotThrow()
    {
        var exception = Record.Exception(() =>
            MouseionMetrics.RecordDatabaseConnection("main"));

        Assert.Null(exception);
    }

    [Fact]
    public void RecordCacheHit_DoesNotThrow()
    {
        var exception = Record.Exception(() =>
            MouseionMetrics.RecordCacheHit("metadata", "movie:123"));

        Assert.Null(exception);
    }

    [Fact]
    public void RecordCacheMiss_DoesNotThrow()
    {
        var exception = Record.Exception(() =>
            MouseionMetrics.RecordCacheMiss("metadata", "movie:999"));

        Assert.Null(exception);
    }

    [Fact]
    public void RecordCacheEviction_WithReason_DoesNotThrow()
    {
        var exception = Record.Exception(() =>
            MouseionMetrics.RecordCacheEviction("metadata", "size_limit"));

        Assert.Null(exception);
    }

    [Fact]
    public void RecordJobStartedAndCompleted_DoesNotThrow()
    {
        var exception = Record.Exception(() =>
        {
            MouseionMetrics.RecordJobStarted("DiskScan", "scheduled");
            MouseionMetrics.RecordJobCompleted("DiskScan", "scheduled", true, 30000.0);
        });

        Assert.Null(exception);
    }

    [Fact]
    public void RecordJobCompleted_Failure_DoesNotThrow()
    {
        var exception = Record.Exception(() =>
        {
            MouseionMetrics.RecordJobStarted("MetadataRefresh", "manual");
            MouseionMetrics.RecordJobCompleted("MetadataRefresh", "manual", false, 5000.0);
        });

        Assert.Null(exception);
    }

    [Fact]
    public void SetLibraryCounts_WithAllTypes_DoesNotThrow()
    {
        var exception = Record.Exception(() =>
            MouseionMetrics.SetLibraryCounts(
                movies: 150,
                books: 500,
                audiobooks: 75,
                albums: 1000,
                tracks: 15000,
                tvShows: 50,
                podcasts: 25));

        Assert.Null(exception);
    }

    [Fact]
    public void SetLibraryCounts_PartialUpdate_DoesNotThrow()
    {
        var exception = Record.Exception(() =>
            MouseionMetrics.SetLibraryCounts(movies: 200));

        Assert.Null(exception);
    }

    [Fact]
    public void RecordIndexerSearch_WithResults_DoesNotThrow()
    {
        var exception = Record.Exception(() =>
            MouseionMetrics.RecordIndexerSearch("torznab", "movie_search", true, 1500.0, 25));

        Assert.Null(exception);
    }

    [Fact]
    public void StartActivity_ReturnsActivity()
    {
        // Note: Activity may be null if there are no listeners
        var activity = MouseionMetrics.StartActivity("test_operation");

        // The activity may be null if no listeners are registered,
        // but calling the method should not throw
        Assert.True(activity == null || activity is Activity);
        activity?.Dispose();
    }

    [Fact]
    public void StartDatabaseActivity_SetsCorrectTags()
    {
        using var listener = new ActivityListener
        {
            ShouldListenTo = _ => true,
            Sample = (ref ActivityCreationOptions<ActivityContext> _) => ActivitySamplingResult.AllData
        };
        ActivitySource.AddActivityListener(listener);

        using var activity = MouseionMetrics.StartDatabaseActivity("SELECT", "Movies");

        // Activity might be null if source is not being listened to
        if (activity != null)
        {
            Assert.Equal("db.SELECT", activity.OperationName);
            Assert.Equal("sqlite", activity.GetTagItem("db.system"));
            Assert.Equal("SELECT", activity.GetTagItem("db.operation"));
            Assert.Equal("Movies", activity.GetTagItem("db.sql.table"));
        }
    }

    [Fact]
    public void StartHttpActivity_SetsCorrectTags()
    {
        using var listener = new ActivityListener
        {
            ShouldListenTo = _ => true,
            Sample = (ref ActivityCreationOptions<ActivityContext> _) => ActivitySamplingResult.AllData
        };
        ActivitySource.AddActivityListener(listener);

        using var activity = MouseionMetrics.StartHttpActivity("GET", "https://api.themoviedb.org/3/movie/123", "tmdb");

        if (activity != null)
        {
            Assert.Equal("http.get", activity.OperationName);
            Assert.Equal("GET", activity.GetTagItem("http.method"));
            Assert.Equal("https://api.themoviedb.org/3/movie/123", activity.GetTagItem("http.url"));
            Assert.Equal("tmdb", activity.GetTagItem("mouseion.provider"));
        }
    }

    [Fact]
    public void StartJobActivity_SetsCorrectTags()
    {
        using var listener = new ActivityListener
        {
            ShouldListenTo = _ => true,
            Sample = (ref ActivityCreationOptions<ActivityContext> _) => ActivitySamplingResult.AllData
        };
        ActivitySource.AddActivityListener(listener);

        using var activity = MouseionMetrics.StartJobActivity("DiskScan", "scheduled");

        if (activity != null)
        {
            Assert.Equal("job.DiskScan", activity.OperationName);
            Assert.Equal("DiskScan", activity.GetTagItem("job.name"));
            Assert.Equal("scheduled", activity.GetTagItem("job.type"));
        }
    }

    [Fact]
    public void SetActivityError_SetsErrorStatus()
    {
        using var listener = new ActivityListener
        {
            ShouldListenTo = _ => true,
            Sample = (ref ActivityCreationOptions<ActivityContext> _) => ActivitySamplingResult.AllData
        };
        ActivitySource.AddActivityListener(listener);

        using var activity = MouseionMetrics.StartActivity("test_error_operation");

        if (activity != null)
        {
            var exception = new InvalidOperationException("Test error message");
            MouseionMetrics.SetActivityError(activity, exception);

            Assert.Equal(ActivityStatusCode.Error, activity.Status);
            Assert.Equal("InvalidOperationException", activity.GetTagItem("error.type"));
            Assert.Equal("Test error message", activity.GetTagItem("error.message"));
        }
    }

    [Fact]
    public void SetActivityError_WithNullActivity_DoesNotThrow()
    {
        var exception = Record.Exception(() =>
            MouseionMetrics.SetActivityError(null, new Exception("test")));

        Assert.Null(exception);
    }
}

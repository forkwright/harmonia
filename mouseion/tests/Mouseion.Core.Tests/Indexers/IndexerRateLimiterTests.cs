// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Moq;
using Mouseion.Core.Indexers.RateLimiting;

namespace Mouseion.Core.Tests.Indexers;

public class IndexerRateLimiterTests
{
    private readonly Mock<IIndexerRateLimitRepository> _rateLimitRepoMock;
    private readonly Mock<IIndexerRequestLogRepository> _requestLogRepoMock;
    private readonly Mock<ILogger<IndexerRateLimiter>> _loggerMock;
    private readonly IndexerRateLimiter _rateLimiter;

    public IndexerRateLimiterTests()
    {
        _rateLimitRepoMock = new Mock<IIndexerRateLimitRepository>();
        _requestLogRepoMock = new Mock<IIndexerRequestLogRepository>();
        _loggerMock = new Mock<ILogger<IndexerRateLimiter>>();

        _rateLimiter = new IndexerRateLimiter(
            _rateLimitRepoMock.Object,
            _requestLogRepoMock.Object,
            _loggerMock.Object);
    }

    [Fact]
    public void CanRequest_NoConfig_ReturnsAllowed()
    {
        _rateLimitRepoMock.Setup(r => r.GetByName("test-indexer")).Returns((IndexerRateLimit?)null);

        var (allowed, reason) = _rateLimiter.CanRequest("test-indexer");

        Assert.True(allowed);
        Assert.Null(reason);
    }

    [Fact]
    public void CanRequest_Disabled_ReturnsNotAllowed()
    {
        _rateLimitRepoMock.Setup(r => r.GetByName("test-indexer"))
            .Returns(new IndexerRateLimit { IndexerName = "test-indexer", Enabled = false });

        var (allowed, reason) = _rateLimiter.CanRequest("test-indexer");

        Assert.False(allowed);
        Assert.Contains("disabled", reason, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void CanRequest_InBackoff_ReturnsNotAllowed()
    {
        _rateLimitRepoMock.Setup(r => r.GetByName("test-indexer"))
            .Returns(new IndexerRateLimit
            {
                IndexerName = "test-indexer",
                Enabled = true,
                BackoffUntil = DateTime.UtcNow.AddMinutes(5)
            });

        var (allowed, reason) = _rateLimiter.CanRequest("test-indexer");

        Assert.False(allowed);
        Assert.Contains("backoff", reason, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void CanRequest_AtCapacity_ReturnsNotAllowed()
    {
        _rateLimitRepoMock.Setup(r => r.GetByName("test-indexer"))
            .Returns(new IndexerRateLimit
            {
                IndexerName = "test-indexer",
                Enabled = true,
                MaxRequestsPerHour = 10
            });

        _requestLogRepoMock.Setup(r => r.CountSince("test-indexer", It.IsAny<DateTime>()))
            .Returns(10);

        var (allowed, reason) = _rateLimiter.CanRequest("test-indexer");

        Assert.False(allowed);
        Assert.Contains("capacity", reason, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void CanRequest_UnderCapacity_ReturnsAllowed()
    {
        _rateLimitRepoMock.Setup(r => r.GetByName("test-indexer"))
            .Returns(new IndexerRateLimit
            {
                IndexerName = "test-indexer",
                Enabled = true,
                MaxRequestsPerHour = 100
            });

        _requestLogRepoMock.Setup(r => r.CountSince("test-indexer", It.IsAny<DateTime>()))
            .Returns(50);

        var (allowed, reason) = _rateLimiter.CanRequest("test-indexer");

        Assert.True(allowed);
        Assert.Null(reason);
    }

    [Fact]
    public void RecordError_SetsBackoff()
    {
        _rateLimitRepoMock.Setup(r => r.GetByName("test-indexer"))
            .Returns(new IndexerRateLimit
            {
                IndexerName = "test-indexer",
                Enabled = true,
                BackoffMultiplier = 1
            });

        _rateLimiter.RecordError("test-indexer", 429, "Too Many Requests");

        _rateLimitRepoMock.Verify(r => r.Upsert(It.Is<IndexerRateLimit>(rl =>
            rl.BackoffUntil.HasValue &&
            rl.BackoffMultiplier == 2 &&
            rl.LastErrorCode == 429)), Times.Once);
    }

    [Fact]
    public void RecordError_EscalatesBackoff()
    {
        _rateLimitRepoMock.Setup(r => r.GetByName("test-indexer"))
            .Returns(new IndexerRateLimit
            {
                IndexerName = "test-indexer",
                Enabled = true,
                BackoffMultiplier = 3 // Already at stage 3 (30min)
            });

        _rateLimiter.RecordError("test-indexer", 503, "Service Unavailable");

        _rateLimitRepoMock.Verify(r => r.Upsert(It.Is<IndexerRateLimit>(rl =>
            rl.BackoffMultiplier == 4)), Times.Once);
    }

    [Fact]
    public void ClearBackoff_ResetsState()
    {
        _rateLimitRepoMock.Setup(r => r.GetByName("test-indexer"))
            .Returns(new IndexerRateLimit
            {
                IndexerName = "test-indexer",
                BackoffUntil = DateTime.UtcNow.AddHours(4),
                BackoffMultiplier = 4,
                LastErrorCode = 429
            });

        _rateLimiter.ClearBackoff("test-indexer");

        _rateLimitRepoMock.Verify(r => r.Upsert(It.Is<IndexerRateLimit>(rl =>
            !rl.BackoffUntil.HasValue &&
            rl.BackoffMultiplier == 1 &&
            !rl.LastErrorCode.HasValue)), Times.Once);
    }

    [Fact]
    public void GetHealthStatus_ReturnsAllIndexers()
    {
        _rateLimitRepoMock.Setup(r => r.GetAll())
            .Returns(new List<IndexerRateLimit>
            {
                new() { IndexerName = "torznab", MaxRequestsPerHour = 100, Enabled = true },
                new() { IndexerName = "gazelle", MaxRequestsPerHour = 50, Enabled = true }
            });

        _requestLogRepoMock.Setup(r => r.CountSince(It.IsAny<string>(), It.IsAny<DateTime>()))
            .Returns(25);

        var statuses = _rateLimiter.GetHealthStatus();

        Assert.Equal(2, statuses.Count);
        Assert.Equal("torznab", statuses[0].IndexerName);
        Assert.Equal(25, statuses[0].RequestsUsed);
        Assert.Equal(75, statuses[0].RequestsRemaining);
    }

    [Fact]
    public void Configure_SetsMaxRequests()
    {
        _rateLimitRepoMock.Setup(r => r.GetByName("test-indexer"))
            .Returns(new IndexerRateLimit { IndexerName = "test-indexer", MaxRequestsPerHour = 100 });

        _rateLimiter.Configure("test-indexer", 50);

        _rateLimitRepoMock.Verify(r => r.Upsert(It.Is<IndexerRateLimit>(rl =>
            rl.MaxRequestsPerHour == 50)), Times.Once);
    }

    [Fact]
    public void RecordRequest_ClearsBackoffOnSuccess()
    {
        _rateLimitRepoMock.Setup(r => r.GetByName("test-indexer"))
            .Returns(new IndexerRateLimit
            {
                IndexerName = "test-indexer",
                BackoffMultiplier = 3,
                BackoffUntil = DateTime.UtcNow.AddMinutes(-1) // Expired
            });

        _rateLimiter.RecordRequest("test-indexer", 200, 150, 10, "test query");

        _requestLogRepoMock.Verify(r => r.Log(It.IsAny<IndexerRequestLog>()), Times.Once);
        _rateLimitRepoMock.Verify(r => r.Upsert(It.Is<IndexerRateLimit>(rl =>
            rl.BackoffMultiplier == 1 && !rl.BackoffUntil.HasValue)), Times.Once);
    }
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Xunit;
using Moq;
using Microsoft.Extensions.Logging;
using Mouseion.Core.Subtitles;
using Mouseion.Core.Movies;
using Mouseion.Core.MediaFiles;

namespace Mouseion.Core.Tests.Subtitles;

public class SubtitleServiceTests
{
    private readonly Mock<IOpenSubtitlesProxy> _mockProxy;
    private readonly Mock<IMovieRepository> _mockMovieRepo;
    private readonly Mock<IMediaFileRepository> _mockFileRepo;
    private readonly Mock<ILogger<SubtitleService>> _mockLogger;
    private readonly SubtitleService _service;

    public SubtitleServiceTests()
    {
        _mockProxy = new Mock<IOpenSubtitlesProxy>();
        _mockMovieRepo = new Mock<IMovieRepository>();
        _mockFileRepo = new Mock<IMediaFileRepository>();
        _mockLogger = new Mock<ILogger<SubtitleService>>();

        _service = new SubtitleService(
            _mockProxy.Object,
            _mockMovieRepo.Object,
            _mockFileRepo.Object,
            _mockLogger.Object);
    }

    [Fact]
    public async Task SearchForMovieAsync_MovieNotFound_ShouldReturnEmptyList()
    {
        _mockMovieRepo.Setup(r => r.FindAsync(999, It.IsAny<CancellationToken>()))
            .ReturnsAsync((Movie?)null);

        var results = await _service.SearchForMovieAsync(999);

        Assert.Empty(results);
    }

    [Fact]
    public async Task SearchForMovieAsync_NoMediaFiles_WithImdbId_ShouldSearchByImdb()
    {
        var movie = new Movie
        {
            Id = 1,
            Title = "Test Movie",
            ImdbId = "tt1234567",
            Path = "/movies/test.mkv",
            RootFolderPath = "/movies"
        };

        _mockMovieRepo.Setup(r => r.FindAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(movie);

        _mockFileRepo.Setup(r => r.GetByMediaItemIdAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new List<MediaFile>());

        var expectedResults = new List<SubtitleSearchResult>
        {
            new SubtitleSearchResult { FileId = 123, FileName = "test.srt", Language = "en" }
        };

        _mockProxy.Setup(p => p.SearchByImdbAsync("tt1234567", null, It.IsAny<CancellationToken>()))
            .ReturnsAsync(expectedResults);

        var results = await _service.SearchForMovieAsync(1);

        Assert.Single(results);
        Assert.Equal(123, results[0].FileId);
        _mockProxy.Verify(p => p.SearchByImdbAsync("tt1234567", null, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task DownloadSubtitleAsync_MovieNotFound_ShouldThrowArgumentException()
    {
        _mockMovieRepo.Setup(r => r.FindAsync(999, It.IsAny<CancellationToken>()))
            .ReturnsAsync((Movie?)null);

        var ex = await Assert.ThrowsAsync<ArgumentException>(() =>
            _service.DownloadSubtitleAsync(123, 999));

        Assert.Contains("Movie 999 not found", ex.Message);
    }

    [Fact]
    public async Task DownloadSubtitleAsync_DownloadInfoNotFound_ShouldThrowInvalidOperationException()
    {
        var movie = new Movie
        {
            Id = 1,
            Title = "Test Movie",
            Path = "/movies/test.mkv",
            RootFolderPath = "/movies"
        };

        _mockMovieRepo.Setup(r => r.FindAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(movie);

        _mockProxy.Setup(p => p.GetDownloadInfoAsync(999, It.IsAny<CancellationToken>()))
            .ReturnsAsync((SubtitleDownloadInfo?)null);

        var ex = await Assert.ThrowsAsync<InvalidOperationException>(() =>
            _service.DownloadSubtitleAsync(999, 1));

        Assert.Contains("Failed to get download info", ex.Message);
    }
}

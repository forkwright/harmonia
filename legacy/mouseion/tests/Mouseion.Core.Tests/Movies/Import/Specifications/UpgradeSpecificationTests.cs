// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging.Abstractions;
using Moq;
using Mouseion.Core.Movies;
using Mouseion.Core.Movies.Import.Specifications;
using Mouseion.Core.Qualities;

namespace Mouseion.Core.Tests.Movies.Import.Specifications;

public class UpgradeSpecificationTests
{
    private readonly Mock<IMovieFileRepository> _movieFileRepositoryMock;
    private readonly UpgradeSpecification _specification;

    public UpgradeSpecificationTests()
    {
        _movieFileRepositoryMock = new Mock<IMovieFileRepository>();
        _specification = new UpgradeSpecification(
            _movieFileRepositoryMock.Object,
            NullLogger<UpgradeSpecification>.Instance);
    }

    [Fact]
    public async Task IsSatisfiedByAsync_should_allow_import_when_no_existing_file()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        // Use real filename that parser can understand
        var filePath = "/movies/Test.Movie.2025.1080p.BluRay.mkv";

        _movieFileRepositoryMock
            .Setup(x => x.FindByMovieIdAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync((MovieFile?)null);

        var result = await _specification.IsSatisfiedByAsync(filePath, movie);

        Assert.Null(result); // No rejection = allowed
    }

    [Fact]
    public async Task IsSatisfiedByAsync_should_allow_upgrade_when_candidate_is_better()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        var filePath = "/movies/Test.Movie.2025.1080p.BluRay.mkv";

        var existingFile = new MovieFile
        {
            Id = 1,
            MovieId = 1,
            Path = "/movies/Test.Movie.2025.720p.HDTV.mkv",
            Quality = new QualityModel { Quality = Quality.HDTV720p, Revision = new Revision(1) }
        };

        _movieFileRepositoryMock
            .Setup(x => x.FindByMovieIdAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(existingFile);

        var result = await _specification.IsSatisfiedByAsync(filePath, movie);

        Assert.Null(result); // No rejection = upgrade allowed
    }

    [Fact]
    public async Task IsSatisfiedByAsync_should_reject_when_candidate_is_not_upgrade()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        var filePath = "/movies/Test.Movie.2025.720p.HDTV.mkv";

        var existingFile = new MovieFile
        {
            Id = 1,
            MovieId = 1,
            Path = "/movies/Test.Movie.2025.1080p.BluRay.mkv",
            Quality = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) }
        };

        _movieFileRepositoryMock
            .Setup(x => x.FindByMovieIdAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(existingFile);

        var result = await _specification.IsSatisfiedByAsync(filePath, movie);

        Assert.NotNull(result);
        Assert.Contains("not an upgrade", result.Message);
    }

    [Fact]
    public async Task IsSatisfiedByAsync_should_allow_when_quality_is_unknown()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        // Unknown quality filename
        var filePath = "/movies/Test.Movie.2025.mkv";

        var result = await _specification.IsSatisfiedByAsync(filePath, movie);

        Assert.Null(result); // Allow when quality cannot be determined
    }

    [Fact]
    public async Task IsSatisfiedByAsync_should_allow_when_existing_file_has_no_quality()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        var filePath = "/movies/Test.Movie.2025.1080p.BluRay.mkv";

        var existingFile = new MovieFile
        {
            Id = 1,
            MovieId = 1,
            Path = "/movies/Test.Movie.2025.720p.HDTV.mkv",
            Quality = null // Legacy data without quality
        };

        _movieFileRepositoryMock
            .Setup(x => x.FindByMovieIdAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(existingFile);

        var result = await _specification.IsSatisfiedByAsync(filePath, movie);

        Assert.Null(result); // Allow when existing quality is unknown
    }

    [Fact]
    public async Task IsSatisfiedByAsync_should_allow_revision_upgrade()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        var filePath = "/movies/Test.Movie.2025.1080p.BluRay.PROPER.mkv";

        var existingFile = new MovieFile
        {
            Id = 1,
            MovieId = 1,
            Path = "/movies/Test.Movie.2025.1080p.BluRay.mkv",
            Quality = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) }
        };

        _movieFileRepositoryMock
            .Setup(x => x.FindByMovieIdAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(existingFile);

        var result = await _specification.IsSatisfiedByAsync(filePath, movie);

        Assert.Null(result); // PROPER is an upgrade
    }

    [Fact]
    public async Task IsSatisfiedByAsync_should_reject_same_quality_and_revision()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        var filePath = "/movies/Test.Movie.2025.1080p.BluRay.mkv";

        var existingFile = new MovieFile
        {
            Id = 1,
            MovieId = 1,
            Path = "/movies/Test.Movie.2025.1080p.BluRay.mkv",
            Quality = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) }
        };

        _movieFileRepositoryMock
            .Setup(x => x.FindByMovieIdAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(existingFile);

        var result = await _specification.IsSatisfiedByAsync(filePath, movie);

        Assert.NotNull(result); // Same quality is not an upgrade
    }

    [Fact]
    public void IsSatisfiedBy_should_work_synchronously()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        var filePath = "/movies/Test.Movie.2025.1080p.BluRay.mkv";

        _movieFileRepositoryMock
            .Setup(x => x.FindByMovieId(1))
            .Returns((MovieFile?)null);

        var result = _specification.IsSatisfiedBy(filePath, movie);

        Assert.Null(result);
    }

    [Fact]
    public void IsSatisfiedBy_should_allow_upgrade_when_candidate_is_better()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        var filePath = "/movies/Test.Movie.2025.1080p.BluRay.mkv";

        var existingFile = new MovieFile
        {
            Id = 1,
            MovieId = 1,
            Path = "/movies/Test.Movie.2025.720p.HDTV.mkv",
            Quality = new QualityModel { Quality = Quality.HDTV720p, Revision = new Revision(1) }
        };

        _movieFileRepositoryMock
            .Setup(x => x.FindByMovieId(1))
            .Returns(existingFile);

        var result = _specification.IsSatisfiedBy(filePath, movie);

        Assert.Null(result); // No rejection = upgrade allowed
    }

    [Fact]
    public void IsSatisfiedBy_should_reject_when_candidate_is_not_upgrade()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        var filePath = "/movies/Test.Movie.2025.720p.HDTV.mkv";

        var existingFile = new MovieFile
        {
            Id = 1,
            MovieId = 1,
            Path = "/movies/Test.Movie.2025.1080p.BluRay.mkv",
            Quality = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) }
        };

        _movieFileRepositoryMock
            .Setup(x => x.FindByMovieId(1))
            .Returns(existingFile);

        var result = _specification.IsSatisfiedBy(filePath, movie);

        Assert.NotNull(result);
        Assert.Contains("not an upgrade", result.Message);
    }

    [Fact]
    public void IsSatisfiedBy_should_allow_when_existing_file_has_no_quality()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        var filePath = "/movies/Test.Movie.2025.1080p.BluRay.mkv";

        var existingFile = new MovieFile
        {
            Id = 1,
            MovieId = 1,
            Path = "/movies/Test.Movie.2025.720p.HDTV.mkv",
            Quality = null // Legacy data without quality
        };

        _movieFileRepositoryMock
            .Setup(x => x.FindByMovieId(1))
            .Returns(existingFile);

        var result = _specification.IsSatisfiedBy(filePath, movie);

        Assert.Null(result); // Allow when existing quality is unknown
    }

    [Fact]
    public void IsSatisfiedBy_should_allow_revision_upgrade()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        var filePath = "/movies/Test.Movie.2025.1080p.BluRay.PROPER.mkv";

        var existingFile = new MovieFile
        {
            Id = 1,
            MovieId = 1,
            Path = "/movies/Test.Movie.2025.1080p.BluRay.mkv",
            Quality = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) }
        };

        _movieFileRepositoryMock
            .Setup(x => x.FindByMovieId(1))
            .Returns(existingFile);

        var result = _specification.IsSatisfiedBy(filePath, movie);

        Assert.Null(result); // PROPER is an upgrade
    }
}

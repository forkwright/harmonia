// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging.Abstractions;
using Mouseion.Core.Movies;
using Mouseion.Core.Movies.Import.Specifications;

namespace Mouseion.Core.Tests.Movies.Import.Specifications;

public class MinimumQualitySpecificationTests
{
    private readonly MinimumQualitySpecification _specification;

    public MinimumQualitySpecificationTests()
    {
        _specification = new MinimumQualitySpecification(
            NullLogger<MinimumQualitySpecification>.Instance);
    }

    [Fact]
    public async Task IsSatisfiedByAsync_should_reject_unknown_quality()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        // Filename with no extension and no quality indicators
        var filePath = "/movies/randomfile";

        var result = await _specification.IsSatisfiedByAsync(filePath, movie);

        Assert.NotNull(result);
        Assert.Contains("Cannot determine quality", result.Message);
    }

    [Fact]
    public async Task IsSatisfiedByAsync_should_allow_known_quality()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        var filePath = "/movies/Test.Movie.2025.1080p.BluRay.mkv";

        var result = await _specification.IsSatisfiedByAsync(filePath, movie);

        Assert.Null(result); // No rejection = allowed
    }

    [Fact]
    public async Task IsSatisfiedByAsync_should_allow_720p()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        var filePath = "/movies/Test.Movie.2025.720p.HDTV.mkv";

        var result = await _specification.IsSatisfiedByAsync(filePath, movie);

        Assert.Null(result);
    }

    [Fact]
    public async Task IsSatisfiedByAsync_should_allow_2160p()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        var filePath = "/movies/Test.Movie.2025.2160p.UHD.BluRay.mkv";

        var result = await _specification.IsSatisfiedByAsync(filePath, movie);

        Assert.Null(result);
    }

    [Fact]
    public async Task IsSatisfiedByAsync_should_allow_HDTV()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        var filePath = "/movies/Test.Movie.2025.HDTV.mkv";

        var result = await _specification.IsSatisfiedByAsync(filePath, movie);

        Assert.Null(result);
    }

    [Fact]
    public async Task IsSatisfiedByAsync_should_allow_WEB_DL()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        var filePath = "/movies/Test.Movie.2025.1080p.WEB-DL.mkv";

        var result = await _specification.IsSatisfiedByAsync(filePath, movie);

        Assert.Null(result);
    }

    [Fact]
    public async Task IsSatisfiedByAsync_should_allow_Remux()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        var filePath = "/movies/Test.Movie.2025.1080p.BluRay.REMUX.mkv";

        var result = await _specification.IsSatisfiedByAsync(filePath, movie);

        Assert.Null(result);
    }

    [Fact]
    public void IsSatisfiedBy_should_reject_unknown_quality()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        var filePath = "/movies/randomfile";

        var result = _specification.IsSatisfiedBy(filePath, movie);

        Assert.NotNull(result);
        Assert.Contains("Cannot determine quality", result.Message);
    }

    [Fact]
    public void IsSatisfiedBy_should_allow_known_quality()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        var filePath = "/movies/Test.Movie.2025.1080p.BluRay.mkv";

        var result = _specification.IsSatisfiedBy(filePath, movie);

        Assert.Null(result);
    }

    [Fact]
    public void IsSatisfiedBy_should_work_synchronously()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Year = 2025 };
        var filePath = "/movies/Test.Movie.2025.720p.HDTV.mkv";

        var result = _specification.IsSatisfiedBy(filePath, movie);

        Assert.Null(result);
    }
}

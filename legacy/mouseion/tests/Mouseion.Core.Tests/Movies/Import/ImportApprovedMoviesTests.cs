// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging.Abstractions;
using Moq;
using Mouseion.Core.MediaFiles.Import;
using Mouseion.Core.Movies;
using Mouseion.Core.Movies.Import;
using Mouseion.Core.Movies.Organization;

namespace Mouseion.Core.Tests.Movies.Import;

public class ImportApprovedMoviesTests
{
    private readonly Mock<IMovieFileRepository> _movieFileRepositoryMock;
    private readonly Mock<IFileImportService> _fileImportServiceMock;
    private readonly ImportApprovedMovies _service;

    public ImportApprovedMoviesTests()
    {
        _movieFileRepositoryMock = new Mock<IMovieFileRepository>();
        _fileImportServiceMock = new Mock<IFileImportService>();

        _service = new ImportApprovedMovies(
            _movieFileRepositoryMock.Object,
            _fileImportServiceMock.Object,
            NullLogger<ImportApprovedMovies>.Instance);
    }

    [Fact]
    public async Task ImportAsync_with_approved_decisions_should_import_files()
    {
        var movie = new Movie
        {
            Id = 1,
            Title = "Test Movie",
            Year = 2025,
            Path = "/movies/test-movie"
        };

        var decision = new MovieImportDecision("/downloads/test-movie.mkv", movie);

        _fileImportServiceMock
            .Setup(x => x.ImportFileAsync(
                It.IsAny<string>(),
                It.IsAny<string>(),
                null,
                true))
            .ReturnsAsync((string src, string dst, FileStrategy? _, bool __) =>
                ImportResult.Success(dst, FileStrategy.Hardlink, Mouseion.Common.Disk.TransferMode.HardLink));

        _movieFileRepositoryMock
            .Setup(x => x.InsertAsync(It.IsAny<MovieFile>(), default))
            .ReturnsAsync((MovieFile mf, CancellationToken _) => mf);

        var results = await _service.ImportAsync(new List<MovieImportDecision> { decision });

        Assert.Single(results);
        Assert.True(results[0].Success);
        _fileImportServiceMock.Verify(x => x.ImportFileAsync(
            It.IsAny<string>(),
            It.IsAny<string>(),
            null,
            true), Times.Once);
        _movieFileRepositoryMock.Verify(x => x.InsertAsync(
            It.Is<MovieFile>(mf => mf.MovieId == 1),
            default), Times.Once);
    }

    [Fact]
    public async Task ImportAsync_should_handle_import_service_failures()
    {
        var movie = new Movie
        {
            Id = 1,
            Title = "Test Movie",
            Year = 2025,
            Path = "/movies/test-movie"
        };

        var decision = new MovieImportDecision("/downloads/test-movie.mkv", movie);

        _fileImportServiceMock
            .Setup(x => x.ImportFileAsync(
                It.IsAny<string>(),
                It.IsAny<string>(),
                null,
                true))
            .ReturnsAsync(ImportResult.Failure("Disk full"));

        var results = await _service.ImportAsync(new List<MovieImportDecision> { decision });

        Assert.Single(results);
        Assert.False(results[0].Success);
        Assert.Equal("Disk full", results[0].ErrorMessage);
        _movieFileRepositoryMock.Verify(x => x.InsertAsync(It.IsAny<MovieFile>(), default), Times.Never);
    }

    [Fact]
    public async Task ImportAsync_should_reject_unapproved_decisions()
    {
        var movie = new Movie
        {
            Id = 1,
            Title = "Test Movie",
            Year = 2025,
            Path = "/movies/test-movie"
        };

        var rejection = new ImportRejection(ImportRejectionReason.UnableToParse, "Cannot parse quality");
        var decision = new MovieImportDecision("/downloads/test-movie.mkv", movie, rejection);

        var results = await _service.ImportAsync(new List<MovieImportDecision> { decision });

        Assert.Single(results);
        Assert.False(results[0].Success);
        Assert.Contains("Cannot parse quality", results[0].ErrorMessage);
        _fileImportServiceMock.Verify(x => x.ImportFileAsync(
            It.IsAny<string>(),
            It.IsAny<string>(),
            It.IsAny<FileStrategy?>(),
            It.IsAny<bool>()), Times.Never);
    }

    [Fact]
    public async Task ImportAsync_should_handle_multiple_decisions()
    {
        var movie1 = new Movie { Id = 1, Title = "Movie 1", Year = 2025, Path = "/movies/movie1" };
        var movie2 = new Movie { Id = 2, Title = "Movie 2", Year = 2024, Path = "/movies/movie2" };

        var decision1 = new MovieImportDecision("/downloads/movie1.mkv", movie1);
        var decision2 = new MovieImportDecision("/downloads/movie2.mkv", movie2);

        _fileImportServiceMock
            .Setup(x => x.ImportFileAsync(
                It.IsAny<string>(),
                It.IsAny<string>(),
                null,
                true))
            .ReturnsAsync((string src, string dst, FileStrategy? _, bool __) =>
                ImportResult.Success(dst, FileStrategy.Hardlink, Mouseion.Common.Disk.TransferMode.HardLink));

        _movieFileRepositoryMock
            .Setup(x => x.InsertAsync(It.IsAny<MovieFile>(), default))
            .ReturnsAsync((MovieFile mf, CancellationToken _) => mf);

        var results = await _service.ImportAsync(new List<MovieImportDecision> { decision1, decision2 });

        Assert.Equal(2, results.Count);
        Assert.All(results, r => Assert.True(r.Success));
        _fileImportServiceMock.Verify(x => x.ImportFileAsync(
            It.IsAny<string>(),
            It.IsAny<string>(),
            null,
            true), Times.Exactly(2));
        _movieFileRepositoryMock.Verify(x => x.InsertAsync(It.IsAny<MovieFile>(), default), Times.Exactly(2));
    }

    [Fact]
    public async Task ImportAsync_should_continue_on_individual_failures()
    {
        var movie1 = new Movie { Id = 1, Title = "Movie 1", Year = 2025, Path = "/movies/movie1" };
        var movie2 = new Movie { Id = 2, Title = "Movie 2", Year = 2024, Path = "/movies/movie2" };

        var decision1 = new MovieImportDecision("/downloads/movie1.mkv", movie1);
        var decision2 = new MovieImportDecision("/downloads/movie2.mkv", movie2);

        var callCount = 0;
        _fileImportServiceMock
            .Setup(x => x.ImportFileAsync(
                It.IsAny<string>(),
                It.IsAny<string>(),
                null,
                true))
            .ReturnsAsync((string src, string dst, FileStrategy? _, bool __) =>
            {
                callCount++;
                if (callCount == 1)
                {
                    return ImportResult.Failure("Failed to import");
                }
                return ImportResult.Success(dst, FileStrategy.Copy, Mouseion.Common.Disk.TransferMode.Copy);
            });

        _movieFileRepositoryMock
            .Setup(x => x.InsertAsync(It.IsAny<MovieFile>(), default))
            .ReturnsAsync((MovieFile mf, CancellationToken _) => mf);

        var results = await _service.ImportAsync(new List<MovieImportDecision> { decision1, decision2 });

        Assert.Equal(2, results.Count);
        Assert.False(results[0].Success);
        Assert.True(results[1].Success);
        _movieFileRepositoryMock.Verify(x => x.InsertAsync(It.IsAny<MovieFile>(), default), Times.Once);
    }

    [Fact]
    public async Task ImportAsync_should_handle_IOException()
    {
        var movie = new Movie
        {
            Id = 1,
            Title = "Test Movie",
            Year = 2025,
            Path = "/movies/test-movie"
        };

        var decision = new MovieImportDecision("/downloads/test-movie.mkv", movie);

        _fileImportServiceMock
            .Setup(x => x.ImportFileAsync(
                It.IsAny<string>(),
                It.IsAny<string>(),
                null,
                true))
            .ThrowsAsync(new IOException("Disk read error"));

        var results = await _service.ImportAsync(new List<MovieImportDecision> { decision });

        Assert.Single(results);
        Assert.False(results[0].Success);
        Assert.Equal("Disk read error", results[0].ErrorMessage);
        _movieFileRepositoryMock.Verify(x => x.InsertAsync(It.IsAny<MovieFile>(), default), Times.Never);
    }

    [Fact]
    public async Task ImportAsync_should_handle_InvalidOperationException()
    {
        var movie = new Movie
        {
            Id = 1,
            Title = "Test Movie",
            Year = 2025,
            Path = "/movies/test-movie"
        };

        var decision = new MovieImportDecision("/downloads/test-movie.mkv", movie);

        _fileImportServiceMock
            .Setup(x => x.ImportFileAsync(
                It.IsAny<string>(),
                It.IsAny<string>(),
                null,
                true))
            .ReturnsAsync((string src, string dst, FileStrategy? _, bool __) =>
                ImportResult.Success(dst, FileStrategy.Hardlink, Mouseion.Common.Disk.TransferMode.HardLink));

        _movieFileRepositoryMock
            .Setup(x => x.InsertAsync(It.IsAny<MovieFile>(), default))
            .ThrowsAsync(new InvalidOperationException("Database constraint violation"));

        var results = await _service.ImportAsync(new List<MovieImportDecision> { decision });

        Assert.Single(results);
        Assert.False(results[0].Success);
        Assert.Equal("Database constraint violation", results[0].ErrorMessage);
    }

    [Fact]
    public async Task ImportAsync_should_handle_UnauthorizedAccessException()
    {
        var movie = new Movie
        {
            Id = 1,
            Title = "Test Movie",
            Year = 2025,
            Path = "/movies/test-movie"
        };

        var decision = new MovieImportDecision("/downloads/test-movie.mkv", movie);

        _fileImportServiceMock
            .Setup(x => x.ImportFileAsync(
                It.IsAny<string>(),
                It.IsAny<string>(),
                null,
                true))
            .ThrowsAsync(new UnauthorizedAccessException("Permission denied"));

        var results = await _service.ImportAsync(new List<MovieImportDecision> { decision });

        Assert.Single(results);
        Assert.False(results[0].Success);
        Assert.Equal("Permission denied", results[0].ErrorMessage);
        _movieFileRepositoryMock.Verify(x => x.InsertAsync(It.IsAny<MovieFile>(), default), Times.Never);
    }
}

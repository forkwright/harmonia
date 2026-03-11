// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging.Abstractions;
using Moq;
using Mouseion.Core.Movies;
using Mouseion.Core.Movies.Calendar;

namespace Mouseion.Core.Tests.Movies;

public class MovieCalendarServiceTests
{
    private readonly Mock<IMovieRepository> _movieRepositoryMock;
    private readonly Mock<IMovieFileRepository> _movieFileRepositoryMock;
    private readonly MovieCalendarService _calendarService;

    public MovieCalendarServiceTests()
    {
        _movieRepositoryMock = new Mock<IMovieRepository>();
        _movieFileRepositoryMock = new Mock<IMovieFileRepository>();
        _calendarService = new MovieCalendarService(
            _movieRepositoryMock.Object,
            _movieFileRepositoryMock.Object,
            NullLogger<MovieCalendarService>.Instance);
    }

    [Fact]
    public async Task GetCalendarEntriesAsync_should_include_cinema_releases()
    {
        var startDate = new DateTime(2025, 1, 1);
        var endDate = new DateTime(2025, 1, 31);
        var movies = new List<Movie>
        {
            new Movie
            {
                Id = 1,
                Title = "Test Movie",
                Year = 2025,
                InCinemas = new DateTime(2025, 1, 15),
                Monitored = true
            }
        };

        _movieRepositoryMock
            .Setup(x => x.GetMoviesBetweenDatesAsync(startDate, endDate, false, It.IsAny<CancellationToken>()))
            .ReturnsAsync(movies);

        var result = await _calendarService.GetCalendarEntriesAsync(startDate, endDate);

        Assert.Single(result);
        Assert.Equal("Test Movie", result[0].Title);
        Assert.Equal("Cinema", result[0].ReleaseType);
        Assert.Equal(new DateTime(2025, 1, 15), result[0].ReleaseDate);
        Assert.True(result[0].Monitored);
        Assert.False(result[0].HasFile);
    }

    [Fact]
    public async Task GetCalendarEntriesAsync_should_include_digital_releases()
    {
        var startDate = new DateTime(2025, 1, 1);
        var endDate = new DateTime(2025, 1, 31);
        var movies = new List<Movie>
        {
            new Movie
            {
                Id = 1,
                Title = "Test Movie",
                Year = 2025,
                DigitalRelease = new DateTime(2025, 1, 20),
                Monitored = true
            }
        };

        _movieRepositoryMock
            .Setup(x => x.GetMoviesBetweenDatesAsync(startDate, endDate, false, It.IsAny<CancellationToken>()))
            .ReturnsAsync(movies);

        var result = await _calendarService.GetCalendarEntriesAsync(startDate, endDate);

        Assert.Single(result);
        Assert.Equal("Test Movie", result[0].Title);
        Assert.Equal("Digital", result[0].ReleaseType);
        Assert.Equal(new DateTime(2025, 1, 20), result[0].ReleaseDate);
        Assert.True(result[0].Monitored);
        Assert.False(result[0].HasFile);
    }

    [Fact]
    public async Task GetCalendarEntriesAsync_should_include_physical_releases()
    {
        var startDate = new DateTime(2025, 1, 1);
        var endDate = new DateTime(2025, 1, 31);
        var movies = new List<Movie>
        {
            new Movie
            {
                Id = 1,
                Title = "Test Movie",
                Year = 2025,
                PhysicalRelease = new DateTime(2025, 1, 25),
                Monitored = true
            }
        };

        _movieRepositoryMock
            .Setup(x => x.GetMoviesBetweenDatesAsync(startDate, endDate, false, It.IsAny<CancellationToken>()))
            .ReturnsAsync(movies);

        var result = await _calendarService.GetCalendarEntriesAsync(startDate, endDate);

        Assert.Single(result);
        Assert.Equal("Test Movie", result[0].Title);
        Assert.Equal("Physical", result[0].ReleaseType);
        Assert.Equal(new DateTime(2025, 1, 25), result[0].ReleaseDate);
        Assert.True(result[0].Monitored);
        Assert.False(result[0].HasFile);
    }

    [Fact]
    public async Task GetCalendarEntriesAsync_should_include_all_release_types_for_same_movie()
    {
        var startDate = new DateTime(2025, 1, 1);
        var endDate = new DateTime(2025, 1, 31);
        var movies = new List<Movie>
        {
            new Movie
            {
                Id = 1,
                Title = "Test Movie",
                Year = 2025,
                InCinemas = new DateTime(2025, 1, 10),
                DigitalRelease = new DateTime(2025, 1, 20),
                PhysicalRelease = new DateTime(2025, 1, 25),
                Monitored = true
            }
        };

        _movieRepositoryMock
            .Setup(x => x.GetMoviesBetweenDatesAsync(startDate, endDate, false, It.IsAny<CancellationToken>()))
            .ReturnsAsync(movies);

        var result = await _calendarService.GetCalendarEntriesAsync(startDate, endDate);

        Assert.Equal(3, result.Count);
        Assert.Contains(result, x => x.ReleaseType == "Cinema" && x.ReleaseDate == new DateTime(2025, 1, 10));
        Assert.Contains(result, x => x.ReleaseType == "Digital" && x.ReleaseDate == new DateTime(2025, 1, 20));
        Assert.Contains(result, x => x.ReleaseType == "Physical" && x.ReleaseDate == new DateTime(2025, 1, 25));
    }

    [Fact]
    public async Task GetCalendarEntriesAsync_should_exclude_releases_outside_date_range()
    {
        var startDate = new DateTime(2025, 1, 10);
        var endDate = new DateTime(2025, 1, 20);
        var movies = new List<Movie>
        {
            new Movie
            {
                Id = 1,
                Title = "Too Early",
                Year = 2025,
                InCinemas = new DateTime(2025, 1, 5)
            },
            new Movie
            {
                Id = 2,
                Title = "In Range",
                Year = 2025,
                InCinemas = new DateTime(2025, 1, 15)
            },
            new Movie
            {
                Id = 3,
                Title = "Too Late",
                Year = 2025,
                InCinemas = new DateTime(2025, 1, 25)
            }
        };

        _movieRepositoryMock
            .Setup(x => x.GetMoviesBetweenDatesAsync(startDate, endDate, false, It.IsAny<CancellationToken>()))
            .ReturnsAsync(movies);

        var result = await _calendarService.GetCalendarEntriesAsync(startDate, endDate);

        Assert.Single(result);
        Assert.Equal("In Range", result[0].Title);
    }

    [Fact]
    public async Task GetCalendarEntriesAsync_should_exclude_null_release_dates()
    {
        var startDate = new DateTime(2025, 1, 1);
        var endDate = new DateTime(2025, 1, 31);
        var movies = new List<Movie>
        {
            new Movie
            {
                Id = 1,
                Title = "No Releases",
                Year = 2025,
                InCinemas = null,
                DigitalRelease = null,
                PhysicalRelease = null
            }
        };

        _movieRepositoryMock
            .Setup(x => x.GetMoviesBetweenDatesAsync(startDate, endDate, false, It.IsAny<CancellationToken>()))
            .ReturnsAsync(movies);

        var result = await _calendarService.GetCalendarEntriesAsync(startDate, endDate);

        Assert.Empty(result);
    }

    [Fact]
    public void GetCalendarEntries_should_include_cinema_releases()
    {
        var startDate = new DateTime(2025, 1, 1);
        var endDate = new DateTime(2025, 1, 31);
        var movies = new List<Movie>
        {
            new Movie
            {
                Id = 1,
                Title = "Test Movie",
                Year = 2025,
                InCinemas = new DateTime(2025, 1, 15),
                Monitored = true
            }
        };

        _movieRepositoryMock
            .Setup(x => x.GetMoviesBetweenDates(startDate, endDate, false))
            .Returns(movies);

        var result = _calendarService.GetCalendarEntries(startDate, endDate);

        Assert.Single(result);
        Assert.Equal("Test Movie", result[0].Title);
        Assert.Equal("Cinema", result[0].ReleaseType);
        Assert.False(result[0].HasFile);
    }

    [Fact]
    public void GetCalendarEntries_should_include_digital_releases()
    {
        var startDate = new DateTime(2025, 1, 1);
        var endDate = new DateTime(2025, 1, 31);
        var movies = new List<Movie>
        {
            new Movie
            {
                Id = 1,
                Title = "Test Movie",
                Year = 2025,
                DigitalRelease = new DateTime(2025, 1, 15),
                Monitored = true
            }
        };

        _movieRepositoryMock
            .Setup(x => x.GetMoviesBetweenDates(startDate, endDate, false))
            .Returns(movies);

        var result = _calendarService.GetCalendarEntries(startDate, endDate);

        Assert.Single(result);
        Assert.Equal("Test Movie", result[0].Title);
        Assert.Equal("Digital", result[0].ReleaseType);
        Assert.False(result[0].HasFile);
    }

    [Fact]
    public void GetCalendarEntries_should_include_physical_releases()
    {
        var startDate = new DateTime(2025, 1, 1);
        var endDate = new DateTime(2025, 1, 31);
        var movies = new List<Movie>
        {
            new Movie
            {
                Id = 1,
                Title = "Test Movie",
                Year = 2025,
                PhysicalRelease = new DateTime(2025, 1, 15),
                Monitored = true
            }
        };

        _movieRepositoryMock
            .Setup(x => x.GetMoviesBetweenDates(startDate, endDate, false))
            .Returns(movies);

        var result = _calendarService.GetCalendarEntries(startDate, endDate);

        Assert.Single(result);
        Assert.Equal("Test Movie", result[0].Title);
        Assert.Equal("Physical", result[0].ReleaseType);
        Assert.False(result[0].HasFile);
    }

    [Fact]
    public void GetCalendarEntries_should_exclude_releases_on_different_dates()
    {
        var startDate = new DateTime(2025, 1, 10);
        var endDate = new DateTime(2025, 1, 20);
        var movies = new List<Movie>
        {
            new Movie
            {
                Id = 1,
                Title = "Wrong Date",
                Year = 2025,
                InCinemas = new DateTime(2025, 1, 9)
            },
            new Movie
            {
                Id = 2,
                Title = "Correct Date",
                Year = 2025,
                InCinemas = new DateTime(2025, 1, 15)
            }
        };

        _movieRepositoryMock
            .Setup(x => x.GetMoviesBetweenDates(startDate, endDate, false))
            .Returns(movies);

        var result = _calendarService.GetCalendarEntries(startDate, endDate);

        Assert.Single(result);
        Assert.Equal("Correct Date", result[0].Title);
    }

    [Fact]
    public void GetCalendarEntries_should_sort_by_release_date()
    {
        var startDate = new DateTime(2025, 1, 1);
        var endDate = new DateTime(2025, 1, 31);
        var movies = new List<Movie>
        {
            new Movie
            {
                Id = 1,
                Title = "Movie 1",
                Year = 2025,
                InCinemas = new DateTime(2025, 1, 20),
                DigitalRelease = new DateTime(2025, 1, 10)
            },
            new Movie
            {
                Id = 2,
                Title = "Movie 2",
                Year = 2025,
                PhysicalRelease = new DateTime(2025, 1, 5)
            }
        };

        _movieRepositoryMock
            .Setup(x => x.GetMoviesBetweenDates(startDate, endDate, false))
            .Returns(movies);

        var result = _calendarService.GetCalendarEntries(startDate, endDate);

        Assert.Equal(3, result.Count);
        Assert.Equal(new DateTime(2025, 1, 5), result[0].ReleaseDate);
        Assert.Equal(new DateTime(2025, 1, 10), result[1].ReleaseDate);
        Assert.Equal(new DateTime(2025, 1, 20), result[2].ReleaseDate);
    }

    [Fact]
    public async Task HasFile_should_always_be_false()
    {
        var startDate = new DateTime(2025, 1, 1);
        var endDate = new DateTime(2025, 1, 31);
        var movies = new List<Movie>
        {
            new Movie
            {
                Id = 1,
                Title = "Test Movie",
                Year = 2025,
                InCinemas = new DateTime(2025, 1, 15),
                Monitored = true
            }
        };

        _movieRepositoryMock
            .Setup(x => x.GetMoviesBetweenDatesAsync(startDate, endDate, false, It.IsAny<CancellationToken>()))
            .ReturnsAsync(movies);

        var result = await _calendarService.GetCalendarEntriesAsync(startDate, endDate);

        Assert.All(result, x => Assert.False(x.HasFile));
    }

    [Fact]
    public async Task GetCalendarEntriesAsync_should_set_HasFile_true_when_movie_has_file()
    {
        var startDate = new DateTime(2025, 1, 1);
        var endDate = new DateTime(2025, 1, 31);
        var movies = new List<Movie>
        {
            new Movie
            {
                Id = 1,
                Title = "Movie With File",
                Year = 2025,
                InCinemas = new DateTime(2025, 1, 15),
                Monitored = true
            },
            new Movie
            {
                Id = 2,
                Title = "Movie Without File",
                Year = 2025,
                DigitalRelease = new DateTime(2025, 1, 20),
                Monitored = true
            }
        };

        _movieRepositoryMock
            .Setup(x => x.GetMoviesBetweenDatesAsync(startDate, endDate, false, It.IsAny<CancellationToken>()))
            .ReturnsAsync(movies);

        // Movie 1 has a file, Movie 2 does not
        _movieFileRepositoryMock
            .Setup(x => x.FindByMovieIdAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new MovieFile { Id = 1, MovieId = 1, Path = "/movies/movie1.mkv" });

        _movieFileRepositoryMock
            .Setup(x => x.FindByMovieIdAsync(2, It.IsAny<CancellationToken>()))
            .ReturnsAsync((MovieFile?)null);

        var result = await _calendarService.GetCalendarEntriesAsync(startDate, endDate);

        Assert.Equal(2, result.Count);
        var movieWithFile = result.First(x => x.MovieId == 1);
        var movieWithoutFile = result.First(x => x.MovieId == 2);

        Assert.True(movieWithFile.HasFile);
        Assert.False(movieWithoutFile.HasFile);
    }
}

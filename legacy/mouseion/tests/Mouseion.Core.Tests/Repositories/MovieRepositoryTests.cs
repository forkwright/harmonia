// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Movies;

namespace Mouseion.Core.Tests.Repositories;

public class MovieRepositoryTests : RepositoryTestBase
{
    private readonly IMovieRepository _repository;

    public MovieRepositoryTests()
    {
        _repository = new MovieRepository(Database);
    }

    [Fact]
    public async Task AllAsync_ReturnsOnlyMovies()
    {
        var movie1 = CreateMovie("Movie 1", 2023);
        var movie2 = CreateMovie("Movie 2", 2024);
        await _repository.InsertAsync(movie1);
        await _repository.InsertAsync(movie2);

        var result = await _repository.AllAsync();

        Assert.Equal(2, result.Count());
        Assert.All(result, m => Assert.NotNull(m.Title));
    }

    [Fact]
    public void All_ReturnsOnlyMovies()
    {
        var movie1 = CreateMovie("Movie 1", 2023);
        var movie2 = CreateMovie("Movie 2", 2024);
        _repository.Insert(movie1);
        _repository.Insert(movie2);

        var result = _repository.All();

        Assert.Equal(2, result.Count());
        Assert.All(result, m => Assert.NotNull(m.Title));
    }

    [Fact]
    public async Task GetPageAsync_ReturnsPaginatedResults()
    {
        for (int i = 1; i <= 15; i++)
        {
            await _repository.InsertAsync(CreateMovie($"Movie {i}", 2020 + i));
        }

        var page1 = await _repository.GetPageAsync(1, 5);
        var page2 = await _repository.GetPageAsync(2, 5);

        Assert.Equal(5, page1.Count());
        Assert.Equal(5, page2.Count());
        Assert.NotEqual(page1.First().Id, page2.First().Id);
    }

    [Fact]
    public async Task FindAsync_ReturnsMovieById()
    {
        var movie = CreateMovie("Findable Movie", 2024);
        var inserted = await _repository.InsertAsync(movie);

        var found = await _repository.FindAsync(inserted.Id);

        Assert.NotNull(found);
        Assert.Equal("Findable Movie", found.Title);
        Assert.Equal(2024, found.Year);
    }

    [Fact]
    public async Task FindAsync_ReturnsNullWhenNotFound()
    {
        var found = await _repository.FindAsync(999);

        Assert.Null(found);
    }

    [Fact]
    public void Find_ReturnsMovieById()
    {
        var movie = CreateMovie("Findable Movie", 2024);
        var inserted = _repository.Insert(movie);

        var found = _repository.Find(inserted.Id);

        Assert.NotNull(found);
        Assert.Equal("Findable Movie", found.Title);
    }

    [Fact]
    public async Task FindByTmdbIdAsync_ReturnsMatchingMovie()
    {
        var movie = CreateMovie("TMDB Movie", 2024, tmdbId: "12345");
        await _repository.InsertAsync(movie);

        var found = await _repository.FindByTmdbIdAsync("12345");

        Assert.NotNull(found);
        Assert.Equal("12345", found.TmdbId);
        Assert.Equal("TMDB Movie", found.Title);
    }

    [Fact]
    public async Task FindByTmdbIdAsync_ReturnsNullWhenNoMatch()
    {
        var found = await _repository.FindByTmdbIdAsync("99999");

        Assert.Null(found);
    }

    [Fact]
    public void FindByTmdbId_ReturnsMatchingMovie()
    {
        var movie = CreateMovie("TMDB Movie", 2024, tmdbId: "12345");
        _repository.Insert(movie);

        var found = _repository.FindByTmdbId("12345");

        Assert.NotNull(found);
        Assert.Equal("12345", found.TmdbId);
    }

    [Fact]
    public async Task FindByImdbIdAsync_ReturnsMatchingMovie()
    {
        var movie = CreateMovie("IMDB Movie", 2024, imdbId: "tt1234567");
        await _repository.InsertAsync(movie);

        var found = await _repository.FindByImdbIdAsync("tt1234567");

        Assert.NotNull(found);
        Assert.Equal("tt1234567", found.ImdbId);
        Assert.Equal("IMDB Movie", found.Title);
    }

    [Fact]
    public async Task FindByImdbIdAsync_ReturnsNullWhenNoMatch()
    {
        var found = await _repository.FindByImdbIdAsync("tt9999999");

        Assert.Null(found);
    }

    [Fact]
    public void FindByImdbId_ReturnsMatchingMovie()
    {
        var movie = CreateMovie("IMDB Movie", 2024, imdbId: "tt1234567");
        _repository.Insert(movie);

        var found = _repository.FindByImdbId("tt1234567");

        Assert.NotNull(found);
        Assert.Equal("tt1234567", found.ImdbId);
    }

    [Fact]
    public async Task GetByCollectionIdAsync_ReturnsMoviesByCollection()
    {
        var movie1 = CreateMovie("Collection 1 Movie 1", 2023, collectionId: 1);
        var movie2 = CreateMovie("Collection 1 Movie 2", 2024, collectionId: 1);
        var movie3 = CreateMovie("Collection 2 Movie 1", 2024, collectionId: 2);

        await _repository.InsertAsync(movie1);
        await _repository.InsertAsync(movie2);
        await _repository.InsertAsync(movie3);

        var collection1Movies = await _repository.GetByCollectionIdAsync(1);

        Assert.Equal(2, collection1Movies.Count);
        Assert.All(collection1Movies, m => Assert.Equal(1, m.CollectionId));
    }

    [Fact]
    public void GetByCollectionId_ReturnsMoviesByCollection()
    {
        var movie1 = CreateMovie("Collection 1 Movie 1", 2023, collectionId: 1);
        var movie2 = CreateMovie("Collection 1 Movie 2", 2024, collectionId: 1);

        _repository.Insert(movie1);
        _repository.Insert(movie2);

        var collection1Movies = _repository.GetByCollectionId(1);

        Assert.Equal(2, collection1Movies.Count);
    }

    [Fact]
    public async Task GetMonitoredAsync_ReturnsOnlyMonitoredMovies()
    {
        var monitored1 = CreateMovie("Monitored 1", 2023, monitored: true);
        var monitored2 = CreateMovie("Monitored 2", 2024, monitored: true);
        var unmonitored = CreateMovie("Unmonitored", 2024, monitored: false);

        await _repository.InsertAsync(monitored1);
        await _repository.InsertAsync(monitored2);
        await _repository.InsertAsync(unmonitored);

        var monitoredMovies = await _repository.GetMonitoredAsync();

        Assert.Equal(2, monitoredMovies.Count);
        Assert.All(monitoredMovies, m => Assert.True(m.Monitored));
    }

    [Fact]
    public void GetMonitored_ReturnsOnlyMonitoredMovies()
    {
        var monitored1 = CreateMovie("Monitored 1", 2023, monitored: true);
        var unmonitored = CreateMovie("Unmonitored", 2024, monitored: false);

        _repository.Insert(monitored1);
        _repository.Insert(unmonitored);

        var monitoredMovies = _repository.GetMonitored();

        Assert.Single(monitoredMovies);
        Assert.True(monitoredMovies[0].Monitored);
    }

    [Fact]
    public async Task GetMoviesBetweenDatesAsync_ReturnsMoviesInDateRange()
    {
        var movie1 = CreateMovie("Movie 1", 2024);
        movie1.InCinemas = new DateTime(2024, 6, 15);
        var movie2 = CreateMovie("Movie 2", 2024);
        movie2.DigitalRelease = new DateTime(2024, 7, 20);
        var movie3 = CreateMovie("Movie 3", 2024);
        movie3.PhysicalRelease = new DateTime(2024, 12, 31);

        await _repository.InsertAsync(movie1);
        await _repository.InsertAsync(movie2);
        await _repository.InsertAsync(movie3);

        var start = new DateTime(2024, 7, 1);
        var end = new DateTime(2024, 12, 31);
        var movies = await _repository.GetMoviesBetweenDatesAsync(start, end, includeUnmonitored: true);

        Assert.Equal(2, movies.Count);
    }

    [Fact]
    public async Task GetMoviesBetweenDatesAsync_FiltersByMonitored()
    {
        var monitored = CreateMovie("Monitored", 2024, monitored: true);
        monitored.InCinemas = new DateTime(2024, 7, 15);
        var unmonitored = CreateMovie("Unmonitored", 2024, monitored: false);
        unmonitored.InCinemas = new DateTime(2024, 7, 20);

        await _repository.InsertAsync(monitored);
        await _repository.InsertAsync(unmonitored);

        var start = new DateTime(2024, 7, 1);
        var end = new DateTime(2024, 7, 31);
        var movies = await _repository.GetMoviesBetweenDatesAsync(start, end, includeUnmonitored: false);

        Assert.Single(movies);
        Assert.True(movies[0].Monitored);
    }

    [Fact]
    public void GetMoviesBetweenDates_ReturnsMoviesInDateRange()
    {
        var movie1 = CreateMovie("Movie 1", 2024);
        movie1.InCinemas = new DateTime(2024, 6, 15);
        var movie2 = CreateMovie("Movie 2", 2024);
        movie2.DigitalRelease = new DateTime(2024, 7, 20);

        _repository.Insert(movie1);
        _repository.Insert(movie2);

        var start = new DateTime(2024, 7, 1);
        var end = new DateTime(2024, 12, 31);
        var movies = _repository.GetMoviesBetweenDates(start, end, includeUnmonitored: true);

        Assert.Single(movies);
    }

    [Fact]
    public async Task UpdateAsync_ModifiesMovie()
    {
        var movie = CreateMovie("Original Title", 2024);
        var inserted = await _repository.InsertAsync(movie);

        inserted.Title = "Updated Title";
        inserted.TmdbId = "99999";
        await _repository.UpdateAsync(inserted);

        var updated = await _repository.FindAsync(inserted.Id);
        Assert.Equal("Updated Title", updated!.Title);
        Assert.Equal("99999", updated.TmdbId);
    }

    [Fact]
    public async Task DeleteAsync_RemovesMovie()
    {
        var movie = CreateMovie("To Delete", 2024);
        var inserted = await _repository.InsertAsync(movie);

        await _repository.DeleteAsync(inserted.Id);

        var deleted = await _repository.FindAsync(inserted.Id);
        Assert.Null(deleted);
    }

    [Fact]
    public async Task CountAsync_ReturnsCorrectCount()
    {
        await _repository.InsertAsync(CreateMovie("Movie 1", 2023));
        await _repository.InsertAsync(CreateMovie("Movie 2", 2024));

        var count = await _repository.CountAsync();

        Assert.Equal(2, count);
    }
}

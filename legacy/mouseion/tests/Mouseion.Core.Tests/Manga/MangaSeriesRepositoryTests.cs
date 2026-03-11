// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Manga;
using Mouseion.Core.Tests.Repositories;

namespace Mouseion.Core.Tests.Manga;

public class MangaSeriesRepositoryTests : RepositoryTestBase
{
    private readonly MangaSeriesRepository _repository;

    public MangaSeriesRepositoryTests()
    {
        _repository = new MangaSeriesRepository(Database);
    }

    [Fact]
    public async Task InsertAsync_InsertsMangaSeries()
    {
        var series = CreateMangaSeries("One Piece", "a96676e5-8ae2-425e-b549-7f15dd34a6d8");

        var result = await _repository.InsertAsync(series);

        Assert.True(result.Id > 0);
        Assert.Equal("One Piece", result.Title);
    }

    [Fact]
    public async Task FindAsync_ReturnsSeriesById()
    {
        var series = CreateMangaSeries("Naruto", "d8a959f7-648e-4c8d-8f23-f1f3f8e129f3");
        var inserted = await _repository.InsertAsync(series);

        var result = await _repository.FindAsync(inserted.Id);

        Assert.NotNull(result);
        Assert.Equal(inserted.Id, result.Id);
        Assert.Equal("Naruto", result.Title);
    }

    [Fact]
    public async Task FindByMangaDexIdAsync_ReturnsMatchingSeries()
    {
        var mangaDexId = "32d76d19-8a05-4db0-9fc2-e0b0648fe9d0";
        var series = CreateMangaSeries("Chainsaw Man", mangaDexId);
        await _repository.InsertAsync(series);

        var result = await _repository.FindByMangaDexIdAsync(mangaDexId);

        Assert.NotNull(result);
        Assert.Equal(mangaDexId, result.MangaDexId);
    }

    [Fact]
    public async Task FindByMangaDexIdAsync_ReturnsNullWhenNotFound()
    {
        var result = await _repository.FindByMangaDexIdAsync("nonexistent-id");

        Assert.Null(result);
    }

    [Fact]
    public async Task FindByAniListIdAsync_ReturnsMatchingSeries()
    {
        var aniListId = 30013;
        var series = CreateMangaSeries("One Punch Man", null, aniListId);
        await _repository.InsertAsync(series);

        var result = await _repository.FindByAniListIdAsync(aniListId);

        Assert.NotNull(result);
        Assert.Equal(aniListId, result.AniListId);
    }

    [Fact]
    public async Task GetMonitoredAsync_ReturnsOnlyMonitoredSeries()
    {
        var monitoredSeries = CreateMangaSeries("Monitored", "id1", monitored: true);
        var unmonitoredSeries = CreateMangaSeries("Unmonitored", "id2", monitored: false);
        await _repository.InsertAsync(monitoredSeries);
        await _repository.InsertAsync(unmonitoredSeries);

        var result = await _repository.GetMonitoredAsync();

        Assert.Single(result);
        Assert.Equal("Monitored", result[0].Title);
    }

    [Fact]
    public async Task UpdateAsync_ModifiesSeries()
    {
        var series = CreateMangaSeries("Original", "orig-id");
        var inserted = await _repository.InsertAsync(series);

        inserted.Title = "Updated";
        inserted.Monitored = false;
        var result = await _repository.UpdateAsync(inserted);

        Assert.Equal("Updated", result.Title);
        Assert.False(result.Monitored);
    }

    [Fact]
    public async Task DeleteAsync_RemovesSeries()
    {
        var series = CreateMangaSeries("ToDelete", "del-id");
        var inserted = await _repository.InsertAsync(series);

        await _repository.DeleteAsync(inserted.Id);
        var result = await _repository.FindAsync(inserted.Id);

        Assert.Null(result);
    }

    [Fact]
    public void FindByMangaDexId_ReturnsMatchingSeries()
    {
        var mangaDexId = "sync-manga-id";
        var series = CreateMangaSeries("Sync Series", mangaDexId);
        _repository.Insert(series);

        var result = _repository.FindByMangaDexId(mangaDexId);

        Assert.NotNull(result);
        Assert.Equal(mangaDexId, result.MangaDexId);
    }

    private static MangaSeries CreateMangaSeries(
        string title = "Test Series",
        string? mangaDexId = null,
        int? aniListId = null,
        bool monitored = true,
        int qualityProfileId = 1)
    {
        return new MangaSeries
        {
            Title = title,
            MangaDexId = mangaDexId ?? Guid.NewGuid().ToString(),
            AniListId = aniListId,
            Monitored = monitored,
            QualityProfileId = qualityProfileId,
            Added = DateTime.UtcNow
        };
    }
}

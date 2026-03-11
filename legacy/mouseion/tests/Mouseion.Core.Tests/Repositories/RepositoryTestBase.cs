// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Data.Sqlite;
using Mouseion.Core.Books;
using Mouseion.Core.Datastore;
using Mouseion.Core.Datastore.Migration.Framework;
using Mouseion.Core.Movies;
using Mouseion.Core.Music;

namespace Mouseion.Core.Tests.Repositories;

public abstract class RepositoryTestBase : IDisposable
{
    protected IDatabase Database { get; }
    private readonly SqliteConnection _connection;
    private bool _disposed;

    protected RepositoryTestBase()
    {
        var connectionString = $"DataSource=test_{Guid.NewGuid()};Mode=Memory;Cache=Shared";
        _connection = new SqliteConnection(connectionString);
        _connection.Open();

        Database = new Database("test", () =>
        {
            var conn = new SqliteConnection(connectionString);
            conn.Open();
            return conn;
        })
        {
            DatabaseType = DatabaseType.SQLite
        };

        var migrationController = new MigrationController();
        migrationController.Migrate(
            connectionString,
            new MigrationContext(MigrationType.Main),
            DatabaseType.SQLite);
    }

    protected Book CreateBook(
        string title = "Test Book",
        int year = 2024,
        int? authorId = null,
        int? bookSeriesId = null,
        bool monitored = true,
        int qualityProfileId = 1)
    {
        var now = DateTime.UtcNow;
        return new Book
        {
            Title = title,
            Year = year,
            AuthorId = authorId,
            BookSeriesId = bookSeriesId,
            Monitored = monitored,
            QualityProfileId = qualityProfileId,
            Added = now,
            LastModified = now
        };
    }

    protected Movie CreateMovie(
        string title = "Test Movie",
        int year = 2024,
        string? tmdbId = null,
        string? imdbId = null,
        int? collectionId = null,
        bool monitored = true,
        int qualityProfileId = 1)
    {
        var now = DateTime.UtcNow;
        return new Movie
        {
            Title = title,
            Year = year,
            TmdbId = tmdbId,
            ImdbId = imdbId,
            CollectionId = collectionId,
            Monitored = monitored,
            QualityProfileId = qualityProfileId,
            Added = now,
            LastModified = now
        };
    }

    protected Album CreateAlbum(
        string title = "Test Album",
        int? artistId = null,
        string? foreignAlbumId = null,
        string? releaseGroupMbid = null,
        bool monitored = true,
        int qualityProfileId = 1)
    {
        return new Album
        {
            Title = title,
            ArtistId = artistId,
            ForeignAlbumId = foreignAlbumId,
            ReleaseGroupMbid = releaseGroupMbid,
            Monitored = monitored,
            QualityProfileId = qualityProfileId,
            Added = DateTime.UtcNow
        };
    }

    protected Track CreateTrack(
        string title = "Test Track",
        int? albumId = null,
        int? artistId = null,
        string? foreignTrackId = null,
        int trackNumber = 1,
        int discNumber = 1,
        bool monitored = true,
        int qualityProfileId = 1)
    {
        return new Track
        {
            Title = title,
            AlbumId = albumId,
            ArtistId = artistId,
            ForeignTrackId = foreignTrackId,
            TrackNumber = trackNumber,
            DiscNumber = discNumber,
            Monitored = monitored,
            QualityProfileId = qualityProfileId,
            Added = DateTime.UtcNow
        };
    }

    public void Dispose()
    {
        Dispose(true);
        GC.SuppressFinalize(this);
    }

    protected virtual void Dispose(bool disposing)
    {
        if (_disposed)
        {
            return;
        }

        if (disposing)
        {
            _connection?.Close();
            _connection?.Dispose();
        }

        _disposed = true;
    }
}

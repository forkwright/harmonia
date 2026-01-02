// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Music;

public interface IArtistRepository : IBasicRepository<Artist>
{
    Task<Artist?> FindByNameAsync(string name, CancellationToken ct = default);
    Task<Artist?> FindByForeignIdAsync(string foreignArtistId, CancellationToken ct = default);
    Task<Artist?> FindByMusicBrainzIdAsync(string musicBrainzId, CancellationToken ct = default);
    Task<List<Artist>> GetMonitoredAsync(CancellationToken ct = default);
    Task<bool> ArtistExistsAsync(string name, CancellationToken ct = default);

    Artist? FindByName(string name);
    Artist? FindByForeignId(string foreignArtistId);
    Artist? FindByMusicBrainzId(string musicBrainzId);
    List<Artist> GetMonitored();
    bool ArtistExists(string name);
}

public class ArtistRepository : BasicRepository<Artist>, IArtistRepository
{
    public ArtistRepository(IDatabase database)
        : base(database, "Artists")
    {
    }

    public async Task<Artist?> FindByNameAsync(string name, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Artist>(
            "SELECT * FROM \"Artists\" WHERE \"Name\" = @Name",
            new { Name = name }).ConfigureAwait(false);
    }

    public Artist? FindByName(string name)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Artist>(
            "SELECT * FROM \"Artists\" WHERE \"Name\" = @Name",
            new { Name = name });
    }

    public async Task<Artist?> FindByForeignIdAsync(string foreignArtistId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Artist>(
            "SELECT * FROM \"Artists\" WHERE \"ForeignArtistId\" = @ForeignArtistId",
            new { ForeignArtistId = foreignArtistId }).ConfigureAwait(false);
    }

    public Artist? FindByForeignId(string foreignArtistId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Artist>(
            "SELECT * FROM \"Artists\" WHERE \"ForeignArtistId\" = @ForeignArtistId",
            new { ForeignArtistId = foreignArtistId });
    }

    public async Task<Artist?> FindByMusicBrainzIdAsync(string musicBrainzId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Artist>(
            "SELECT * FROM \"Artists\" WHERE \"MusicBrainzId\" = @MusicBrainzId",
            new { MusicBrainzId = musicBrainzId }).ConfigureAwait(false);
    }

    public Artist? FindByMusicBrainzId(string musicBrainzId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Artist>(
            "SELECT * FROM \"Artists\" WHERE \"MusicBrainzId\" = @MusicBrainzId",
            new { MusicBrainzId = musicBrainzId });
    }

    public async Task<List<Artist>> GetMonitoredAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Artist>(
            "SELECT * FROM \"Artists\" WHERE \"Monitored\" = @Monitored",
            new { Monitored = true }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Artist> GetMonitored()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Artist>(
            "SELECT * FROM \"Artists\" WHERE \"Monitored\" = @Monitored",
            new { Monitored = true }).ToList();
    }

    public async Task<bool> ArtistExistsAsync(string name, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var count = await conn.QuerySingleAsync<int>(
            "SELECT COUNT(*) FROM \"Artists\" WHERE \"Name\" = @Name",
            new { Name = name }).ConfigureAwait(false);
        return count > 0;
    }

    public bool ArtistExists(string name)
    {
        using var conn = _database.OpenConnection();
        var count = conn.QuerySingle<int>(
            "SELECT COUNT(*) FROM \"Artists\" WHERE \"Name\" = @Name",
            new { Name = name });
        return count > 0;
    }
}

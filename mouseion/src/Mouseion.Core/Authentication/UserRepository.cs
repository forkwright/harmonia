// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Authentication;

public interface IUserRepository : IBasicRepository<User>
{
    Task<User?> GetByUsernameAsync(string username, CancellationToken ct = default);
    Task<User?> GetByEmailAsync(string email, CancellationToken ct = default);
    Task<List<User>> GetActiveUsersAsync(CancellationToken ct = default);
    Task UpdateLastLoginAsync(int userId, CancellationToken ct = default);
    Task<bool> UsernameExistsAsync(string username, CancellationToken ct = default);
    Task<bool> EmailExistsAsync(string email, CancellationToken ct = default);
}

public class UserRepository : BasicRepository<User>, IUserRepository
{
    public UserRepository(IDatabase database)
        : base(database, "Users")
    {
    }

    public async Task<User?> GetByUsernameAsync(string username, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<User>(
            @"SELECT * FROM ""Users"" WHERE LOWER(""Username"") = LOWER(@Username)",
            new { Username = username }).ConfigureAwait(false);
    }

    public async Task<User?> GetByEmailAsync(string email, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<User>(
            @"SELECT * FROM ""Users"" WHERE LOWER(""Email"") = LOWER(@Email)",
            new { Email = email }).ConfigureAwait(false);
    }

    public async Task<List<User>> GetActiveUsersAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<User>(
            @"SELECT * FROM ""Users"" WHERE ""IsActive"" = 1 ORDER BY ""Username""")
            .ConfigureAwait(false);
        return result.ToList();
    }

    public async Task UpdateLastLoginAsync(int userId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            @"UPDATE ""Users"" SET ""LastLoginAt"" = @Now, ""UpdatedAt"" = @Now WHERE ""Id"" = @Id",
            new { Id = userId, Now = DateTime.UtcNow }).ConfigureAwait(false);
    }

    public async Task<bool> UsernameExistsAsync(string username, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QuerySingleAsync<int>(
            @"SELECT COUNT(*) FROM ""Users"" WHERE LOWER(""Username"") = LOWER(@Username)",
            new { Username = username }).ConfigureAwait(false) > 0;
    }

    public async Task<bool> EmailExistsAsync(string email, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QuerySingleAsync<int>(
            @"SELECT COUNT(*) FROM ""Users"" WHERE LOWER(""Email"") = LOWER(@Email)",
            new { Email = email }).ConfigureAwait(false) > 0;
    }
}

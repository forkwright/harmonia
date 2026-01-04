// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;
using Mouseion.Core.Datastore.Migration.Framework;
using Serilog;

namespace Mouseion.Core.Housekeeping.Tasks;

public class VacuumLogDatabase : IHousekeepingTask
{
    private readonly IDbFactory _dbFactory;
    private readonly ILogger _logger;

    public string Name => "Vacuum Log Database";

    public VacuumLogDatabase(IDbFactory dbFactory, ILogger logger)
    {
        _dbFactory = dbFactory;
        _logger = logger;
    }

    public Task CleanAsync(CancellationToken cancellationToken = default)
    {
        try
        {
            var logDatabase = _dbFactory.Create(MigrationType.Log);
            logDatabase.Vacuum();
            _logger.Debug("Log database vacuumed successfully");
        }
        catch (Exception ex)
        {
            _logger.Warning(ex, "Failed to vacuum log database");
        }

        return Task.CompletedTask;
    }
}

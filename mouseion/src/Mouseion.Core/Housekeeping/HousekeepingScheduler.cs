// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;
using Mouseion.Core.Jobs;
using Serilog;

namespace Mouseion.Core.Housekeeping;

public class HousekeepingScheduler : IScheduledTask
{
    private readonly IEnumerable<IHousekeepingTask> _housekeepingTasks;
    private readonly IDatabase _database;
    private readonly ILogger _logger;

    public string Name => "Housekeeping";
    public TimeSpan Interval => TimeSpan.FromHours(24);

    public HousekeepingScheduler(
        IEnumerable<IHousekeepingTask> housekeepingTasks,
        IDatabase database,
        ILogger logger)
    {
        _housekeepingTasks = housekeepingTasks;
        _database = database;
        _logger = logger;
    }

    public async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        _logger.Information("Running housekeeping tasks");

        foreach (var task in _housekeepingTasks)
        {
            if (cancellationToken.IsCancellationRequested)
            {
                _logger.Information("Housekeeping cancelled");
                break;
            }

            try
            {
                _logger.Debug("Starting housekeeping task: {TaskName}", task.Name);
                await task.CleanAsync(cancellationToken);
                _logger.Debug("Completed housekeeping task: {TaskName}", task.Name);
            }
            catch (Exception ex)
            {
                _logger.Error(ex, "Error running housekeeping task: {TaskName}", task.Name);
            }
        }

        // Vacuum database after cleanup
        try
        {
            _logger.Debug("Vacuuming database after housekeeping");
            _database.Vacuum();
        }
        catch (Exception ex)
        {
            _logger.Error(ex, "Error vacuuming database");
        }

        _logger.Information("Housekeeping tasks completed");
    }
}

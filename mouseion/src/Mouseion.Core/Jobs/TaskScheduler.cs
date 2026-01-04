// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Jobs;

public class TaskScheduler : BackgroundService
{
    private readonly IEnumerable<IScheduledTask> _scheduledTasks;
    private readonly ILogger<TaskScheduler> _logger;
    private readonly Dictionary<string, DateTime> _lastExecution = new();

    public TaskScheduler(
        IEnumerable<IScheduledTask> scheduledTasks,
        ILogger<TaskScheduler> logger)
    {
        _scheduledTasks = scheduledTasks;
        _logger = logger;
    }

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        _logger.LogInformation("Task scheduler started with {Count} tasks", _scheduledTasks.Count());

        while (!stoppingToken.IsCancellationRequested)
        {
            foreach (var task in _scheduledTasks)
            {
                try
                {
                    if (ShouldExecute(task))
                    {
                        _logger.LogDebug("Executing scheduled task: {TaskName}", task.Name);
                        await task.ExecuteAsync(stoppingToken);
                        _lastExecution[task.Name] = DateTime.UtcNow;
                        _logger.LogDebug("Completed scheduled task: {TaskName}", task.Name);
                    }
                }
                catch (Exception ex)
                {
                    _logger.LogError(ex, "Error executing scheduled task: {TaskName}", task.Name);
                }
            }

            // Check every minute
            await Task.Delay(TimeSpan.FromMinutes(1), stoppingToken);
        }

        _logger.LogInformation("Task scheduler stopped");
    }

    private bool ShouldExecute(IScheduledTask task)
    {
        if (!_lastExecution.TryGetValue(task.Name, out var lastRun))
        {
            return true; // First run
        }

        return DateTime.UtcNow - lastRun >= task.Interval;
    }
}

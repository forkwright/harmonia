// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.Jobs;

namespace Mouseion.Core.SmartLists;

/// <summary>
/// Scheduled task that refreshes all Smart Lists that are due based on their configured intervals.
/// Runs hourly; actual refresh timing is controlled by each list's RefreshInterval setting.
/// </summary>
public partial class SmartListRefreshTask : IScheduledTask
{
    private readonly ISmartListService _service;
    private readonly ILogger<SmartListRefreshTask> _logger;

    public string Name => "Smart List Refresh";
    public TimeSpan Interval => TimeSpan.FromHours(1);

    public SmartListRefreshTask(ISmartListService service, ILogger<SmartListRefreshTask> logger)
    {
        _service = service;
        _logger = logger;
    }

    public async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        LogTaskStart();
        var result = await _service.RefreshAllDueAsync(cancellationToken).ConfigureAwait(false);
        LogTaskComplete(result.ListsProcessed, result.ItemsAdded, result.Errors.Count);
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Starting smart list refresh task")]
    private partial void LogTaskStart();

    [LoggerMessage(Level = LogLevel.Information, Message = "Smart list refresh complete: {Lists} lists processed, {Added} items added, {Errors} errors")]
    private partial void LogTaskComplete(int lists, int added, int errors);
}

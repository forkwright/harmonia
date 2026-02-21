// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Download.Clients.NZBGet;

public class NZBGetClient : IDownloadClient
{
    private readonly NZBGetProxy _proxy;
    private readonly NZBGetSettings _settings;
    private readonly ILogger<NZBGetClient> _logger;

    public NZBGetClient(
        NZBGetProxy proxy,
        NZBGetSettings settings,
        ILogger<NZBGetClient> logger)
    {
        _proxy = proxy;
        _settings = settings;
        _logger = logger;
    }

    public string Name => "NZBGet";
    public DownloadProtocol Protocol => DownloadProtocol.Usenet;

    public async Task<bool> TestConnectionAsync(CancellationToken cancellationToken = default)
    {
        return await _proxy.TestConnectionAsync(_settings, cancellationToken);
    }

    public async Task<IEnumerable<DownloadClientItem>> GetItemsAsync(CancellationToken cancellationToken = default)
    {
        var items = new List<DownloadClientItem>();

        var queue = await _proxy.GetQueueAsync(_settings, cancellationToken);
        var history = await _proxy.GetHistoryAsync(_settings, cancellationToken);

        // Active queue items
        foreach (var item in queue)
        {
            if (!string.IsNullOrWhiteSpace(_settings.Category) &&
                !item.Category.Equals(_settings.Category, StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            items.Add(new DownloadClientItem
            {
                DownloadId = item.NzbId.ToString(),
                Category = item.Category,
                Title = item.NzbName,
                TotalSize = item.FileSize,
                RemainingSize = item.RemainingSize,
                OutputPath = item.DestDir,
                Status = MapQueueStatus(item.Status),
                CanMoveFiles = false,
                CanBeRemoved = false
            });
        }

        // History items
        foreach (var item in history)
        {
            if (!string.IsNullOrWhiteSpace(_settings.Category) &&
                !item.Category.Equals(_settings.Category, StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            items.Add(new DownloadClientItem
            {
                DownloadId = item.NzbId.ToString(),
                Category = item.Category,
                Title = item.Name,
                TotalSize = item.FileSize,
                RemainingSize = 0,
                OutputPath = item.DestDir,
                Status = MapHistoryStatus(item),
                CanMoveFiles = item.Status.StartsWith("SUCCESS", StringComparison.OrdinalIgnoreCase),
                CanBeRemoved = true
            });
        }

        return items;
    }

    public async Task<DownloadClientInfo> GetStatusAsync(CancellationToken cancellationToken = default)
    {
        var status = await _proxy.GetStatusAsync(_settings, cancellationToken);

        // NZBGet doesn't directly expose download dir via status.
        // Get it from the first queue or history item.
        var queue = await _proxy.GetQueueAsync(_settings, cancellationToken);
        var firstDir = queue.FirstOrDefault()?.DestDir ?? string.Empty;

        if (string.IsNullOrEmpty(firstDir))
        {
            var history = await _proxy.GetHistoryAsync(_settings, cancellationToken);
            firstDir = history.FirstOrDefault()?.DestDir ?? string.Empty;
        }

        var outputDir = !string.IsNullOrEmpty(firstDir) ? Path.GetDirectoryName(firstDir) ?? firstDir : string.Empty;

        return new DownloadClientInfo
        {
            IsLocalhost = _settings.Host is "127.0.0.1" or "localhost",
            OutputRootFolders = string.IsNullOrEmpty(outputDir) ? new() : new List<string> { outputDir },
            RemovesCompletedDownloads = false
        };
    }

    public async Task RemoveItemAsync(string downloadId, bool deleteData, CancellationToken cancellationToken = default)
    {
        if (!int.TryParse(downloadId, out var nzbId))
        {
            _logger.LogWarning("Invalid NZBGet download ID: {DownloadId}", downloadId);
            return;
        }

        await _proxy.DeleteAsync(nzbId, deleteData, _settings, cancellationToken);
        _logger.LogInformation("Removed NZB {DownloadId} from NZBGet (deleteData: {DeleteData})",
            downloadId, deleteData);
    }

    private static DownloadItemStatus MapQueueStatus(string status)
    {
        return status.ToUpperInvariant() switch
        {
            "DOWNLOADING" or "FETCHING" => DownloadItemStatus.Downloading,
            "PAUSED" => DownloadItemStatus.Paused,
            "QUEUED" or "WAITING" => DownloadItemStatus.Queued,
            "PP_QUEUED" or "LOADING_PARS" or "VERIFYING_SOURCES" or
            "REPAIRING" or "VERIFYING_REPAIRED" or "RENAMING" or
            "UNPACKING" or "MOVING" or "EXECUTING_SCRIPT" => DownloadItemStatus.Downloading,
            _ => DownloadItemStatus.Queued
        };
    }

    private static DownloadItemStatus MapHistoryStatus(NZBGetHistoryItem item)
    {
        if (item.Status.StartsWith("SUCCESS", StringComparison.OrdinalIgnoreCase))
        {
            return DownloadItemStatus.Completed;
        }

        if (item.Status.StartsWith("FAILURE", StringComparison.OrdinalIgnoreCase))
        {
            return DownloadItemStatus.Failed;
        }

        if (item.ParStatus is "FAILURE" or "MANUAL" || item.UnpackStatus == "FAILURE")
        {
            return DownloadItemStatus.Warning;
        }

        return DownloadItemStatus.Warning;
    }
}

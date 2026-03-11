// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Globalization;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Download.Clients.SABnzbd;

public class SABnzbdClient : IDownloadClient
{
    private readonly SABnzbdProxy _proxy;
    private readonly SABnzbdSettings _settings;
    private readonly ILogger<SABnzbdClient> _logger;

    public SABnzbdClient(
        SABnzbdProxy proxy,
        SABnzbdSettings settings,
        ILogger<SABnzbdClient> logger)
    {
        _proxy = proxy;
        _settings = settings;
        _logger = logger;
    }

    public string Name => "SABnzbd";
    public DownloadProtocol Protocol => DownloadProtocol.Usenet;

    public async Task<bool> TestConnectionAsync(CancellationToken cancellationToken = default)
    {
        return await _proxy.TestConnectionAsync(_settings, cancellationToken);
    }

    public async Task<IEnumerable<DownloadClientItem>> GetItemsAsync(CancellationToken cancellationToken = default)
    {
        var items = new List<DownloadClientItem>();

        var queue = await _proxy.GetQueueAsync(_settings, cancellationToken);
        var history = await _proxy.GetHistoryAsync(_settings, cancellationToken: cancellationToken);

        // Queue items (downloading/queued)
        foreach (var slot in queue.Slots)
        {
            if (!string.IsNullOrWhiteSpace(_settings.Category) &&
                !slot.Category.Equals(_settings.Category, StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            var totalMb = ParseDouble(slot.TotalMb);
            var remainingMb = ParseDouble(slot.RemainingMb);

            items.Add(new DownloadClientItem
            {
                DownloadId = slot.NzoId,
                Category = slot.Category,
                Title = slot.FileName,
                TotalSize = (long)(totalMb * 1024 * 1024),
                RemainingSize = (long)(remainingMb * 1024 * 1024),
                RemainingTime = ParseTimeLeft(slot.TimeLeft),
                Status = MapQueueStatus(slot.Status),
                CanMoveFiles = false,
                CanBeRemoved = false
            });
        }

        // History items (completed/failed)
        foreach (var slot in history.Slots)
        {
            if (!string.IsNullOrWhiteSpace(_settings.Category) &&
                !slot.Category.Equals(_settings.Category, StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            items.Add(new DownloadClientItem
            {
                DownloadId = slot.NzoId,
                Category = slot.Category,
                Title = slot.Name,
                TotalSize = slot.Bytes,
                RemainingSize = 0,
                OutputPath = slot.Storage,
                Message = slot.FailMessage,
                Status = MapHistoryStatus(slot.Status),
                CanMoveFiles = slot.Status == "Completed",
                CanBeRemoved = true
            });
        }

        return items;
    }

    public async Task<DownloadClientInfo> GetStatusAsync(CancellationToken cancellationToken = default)
    {
        // SABnzbd doesn't expose config via API in a straightforward way,
        // so we derive output folder from the first completed history item
        var history = await _proxy.GetHistoryAsync(_settings, limit: 1, cancellationToken: cancellationToken);
        var firstCompleted = history.Slots.FirstOrDefault(s => s.Status == "Completed");

        var outputDir = firstCompleted?.Storage ?? string.Empty;
        if (!string.IsNullOrEmpty(outputDir))
        {
            outputDir = Path.GetDirectoryName(outputDir) ?? outputDir;
        }

        return new DownloadClientInfo
        {
            IsLocalhost = _settings.Host is "127.0.0.1" or "localhost",
            OutputRootFolders = string.IsNullOrEmpty(outputDir) ? new() : new List<string> { outputDir },
            RemovesCompletedDownloads = false
        };
    }

    public async Task RemoveItemAsync(string downloadId, bool deleteData, CancellationToken cancellationToken = default)
    {
        await _proxy.DeleteAsync(downloadId, deleteData, _settings, cancellationToken);
        _logger.LogInformation("Removed NZB {DownloadId} from SABnzbd (deleteData: {DeleteData})",
            downloadId, deleteData);
    }

    private static double ParseDouble(string value)
    {
        return double.TryParse(value, NumberStyles.Any, CultureInfo.InvariantCulture, out var result)
            ? result
            : 0;
    }

    private static TimeSpan? ParseTimeLeft(string timeLeft)
    {
        if (string.IsNullOrWhiteSpace(timeLeft))
        {
            return null;
        }

        // Format: "HH:MM:SS" or "0:00:00"
        if (TimeSpan.TryParse(timeLeft, out var result))
        {
            return result;
        }

        return null;
    }

    private static DownloadItemStatus MapQueueStatus(string status)
    {
        return status.ToLowerInvariant() switch
        {
            "downloading" => DownloadItemStatus.Downloading,
            "paused" => DownloadItemStatus.Paused,
            "queued" or "idle" => DownloadItemStatus.Queued,
            "grabbing" or "fetching" => DownloadItemStatus.Queued,
            "repairing" or "verifying" or "extracting" => DownloadItemStatus.Downloading,
            _ => DownloadItemStatus.Queued
        };
    }

    private static DownloadItemStatus MapHistoryStatus(string status)
    {
        return status switch
        {
            "Completed" => DownloadItemStatus.Completed,
            "Failed" => DownloadItemStatus.Failed,
            _ => DownloadItemStatus.Warning
        };
    }
}

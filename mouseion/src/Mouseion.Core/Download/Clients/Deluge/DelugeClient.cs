// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Download.Clients.Deluge;

public class DelugeClient : IDownloadClient
{
    private readonly DelugeProxy _proxy;
    private readonly DelugeSettings _settings;
    private readonly ILogger<DelugeClient> _logger;

    public DelugeClient(
        DelugeProxy proxy,
        DelugeSettings settings,
        ILogger<DelugeClient> logger)
    {
        _proxy = proxy;
        _settings = settings;
        _logger = logger;
    }

    public string Name => "Deluge";
    public DownloadProtocol Protocol => DownloadProtocol.Torrent;

    public async Task<bool> TestConnectionAsync(CancellationToken cancellationToken = default)
    {
        return await _proxy.TestConnectionAsync(_settings, cancellationToken);
    }

    public async Task<IEnumerable<DownloadClientItem>> GetItemsAsync(CancellationToken cancellationToken = default)
    {
        var torrents = await _proxy.GetTorrentsAsync(_settings, cancellationToken);

        return torrents.Values
            .Where(t => string.IsNullOrWhiteSpace(_settings.Category) ||
                        t.Label.Equals(_settings.Category, StringComparison.OrdinalIgnoreCase))
            .Select(t => new DownloadClientItem
            {
                DownloadId = t.Hash.ToUpperInvariant(),
                Category = t.Label,
                Title = t.Name,
                TotalSize = t.TotalSize,
                RemainingSize = t.TotalRemaining,
                RemainingTime = GetRemainingTime(t),
                SeedRatio = t.Ratio,
                OutputPath = t.SavePath,
                Message = t.Message,
                Status = MapStatus(t),
                CanMoveFiles = false,
                CanBeRemoved = t.IsFinished || t.Paused
            });
    }

    public async Task<DownloadClientInfo> GetStatusAsync(CancellationToken cancellationToken = default)
    {
        var torrents = await _proxy.GetTorrentsAsync(_settings, cancellationToken);
        var firstPath = torrents.Values.FirstOrDefault()?.SavePath ?? string.Empty;

        return new DownloadClientInfo
        {
            IsLocalhost = _settings.Host is "127.0.0.1" or "localhost",
            OutputRootFolders = string.IsNullOrEmpty(firstPath)
                ? new()
                : new List<string> { firstPath },
            RemovesCompletedDownloads = false
        };
    }

    public async Task RemoveItemAsync(string downloadId, bool deleteData, CancellationToken cancellationToken = default)
    {
        await _proxy.RemoveTorrentAsync(downloadId.ToLowerInvariant(), deleteData, _settings, cancellationToken);
        _logger.LogInformation("Removed torrent {DownloadId} from Deluge (deleteData: {DeleteData})",
            downloadId, deleteData);
    }

    private static TimeSpan? GetRemainingTime(DelugeTorrent torrent)
    {
        if (torrent.Eta < 0 || torrent.Eta > 365 * 24 * 3600)
        {
            return null;
        }

        return TimeSpan.FromSeconds(torrent.Eta);
    }

    private static DownloadItemStatus MapStatus(DelugeTorrent torrent)
    {
        if (!string.IsNullOrEmpty(torrent.Message) && torrent.State == "Error")
        {
            return DownloadItemStatus.Warning;
        }

        return torrent.State switch
        {
            "Downloading" => DownloadItemStatus.Downloading,
            "Seeding" => DownloadItemStatus.Completed,
            "Paused" => torrent.IsFinished
                ? DownloadItemStatus.Completed
                : DownloadItemStatus.Paused,
            "Checking" or "Moving" => DownloadItemStatus.Queued,
            "Queued" => DownloadItemStatus.Queued,
            "Error" => DownloadItemStatus.Failed,
            "Allocating" => DownloadItemStatus.Queued,
            _ => DownloadItemStatus.Downloading
        };
    }
}

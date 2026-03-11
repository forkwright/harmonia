// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Download.Clients.QBittorrent;

public class QBittorrentClient : IDownloadClient
{
    private readonly QBittorrentProxy _proxy;
    private readonly QBittorrentSettings _settings;
    private readonly ILogger<QBittorrentClient> _logger;

    public QBittorrentClient(
        QBittorrentProxy proxy,
        QBittorrentSettings settings,
        ILogger<QBittorrentClient> logger)
    {
        _proxy = proxy;
        _settings = settings;
        _logger = logger;
    }

    public string Name => "qBittorrent";

    public DownloadProtocol Protocol => DownloadProtocol.Torrent;

    public async Task<bool> TestConnectionAsync(CancellationToken cancellationToken = default)
    {
        return await _proxy.TestConnectionAsync(_settings, cancellationToken);
    }

    public async Task<IEnumerable<DownloadClientItem>> GetItemsAsync(CancellationToken cancellationToken = default)
    {
        var torrents = await _proxy.GetTorrentsAsync(_settings, cancellationToken);
        var config = await _proxy.GetConfigAsync(_settings, cancellationToken);

        var items = new List<DownloadClientItem>();

        foreach (var torrent in torrents)
        {
            var item = new DownloadClientItem
            {
                DownloadId = torrent.Hash.ToUpperInvariant(),
                Category = !string.IsNullOrWhiteSpace(torrent.Category) ? torrent.Category : torrent.Label,
                Title = torrent.Name,
                TotalSize = torrent.Size,
                RemainingSize = (long)(torrent.Size * (1.0 - torrent.Progress)),
                RemainingTime = GetRemainingTime(torrent),
                SeedRatio = torrent.Ratio,
                OutputPath = !string.IsNullOrWhiteSpace(torrent.ContentPath) ? torrent.ContentPath : torrent.SavePath,
                Status = MapTorrentState(torrent.State, config.DhtEnabled),
                CanMoveFiles = false,
                CanBeRemoved = torrent.State is "pausedUP" or "stoppedUP"
            };

            items.Add(item);
        }

        return items;
    }

    public async Task<DownloadClientInfo> GetStatusAsync(CancellationToken cancellationToken = default)
    {
        var config = await _proxy.GetConfigAsync(_settings, cancellationToken);
        var labels = await _proxy.GetLabelsAsync(_settings, cancellationToken);

        var destDir = config.SavePath;

        // Check if category has custom save path
        if (!string.IsNullOrWhiteSpace(_settings.Category) &&
            labels.TryGetValue(_settings.Category, out var label) &&
            !string.IsNullOrWhiteSpace(label.SavePath))
        {
            destDir = label.SavePath;
        }

        return new DownloadClientInfo
        {
            IsLocalhost = _settings.Host is "127.0.0.1" or "localhost",
            OutputRootFolders = new List<string> { destDir },
            RemovesCompletedDownloads = config.MaxRatioEnabled || config.MaxSeedingTimeEnabled
        };
    }

    public async Task RemoveItemAsync(string downloadId, bool deleteData, CancellationToken cancellationToken = default)
    {
        await _proxy.RemoveTorrentAsync(downloadId, deleteData, _settings, cancellationToken);
        _logger.LogInformation("Removed torrent {DownloadId} (deleteData: {DeleteData})", downloadId, deleteData);
    }

    private static TimeSpan? GetRemainingTime(QBittorrentTorrent torrent)
    {
        // qBittorrent returns negative eta or very large values for unknown/queued
        if (torrent.Eta < 0 || torrent.Eta > 365 * 24 * 3600 || torrent.Eta == 8640000)
        {
            return null;
        }

        return TimeSpan.FromSeconds(torrent.Eta);
    }

    private static DownloadItemStatus MapTorrentState(string state, bool dhtEnabled)
    {
        return state switch
        {
            "error" => DownloadItemStatus.Warning,
            "stoppedDL" or "pausedDL" => DownloadItemStatus.Paused,
            "queuedDL" or "checkingDL" or "checkingUP" or "checkingResumeData" => DownloadItemStatus.Queued,
            "pausedUP" or "stoppedUP" or "uploading" or "stalledUP" or "queuedUP" or "forcedUP" => DownloadItemStatus.Completed,
            "stalledDL" => DownloadItemStatus.Warning,
            "missingFiles" => DownloadItemStatus.Warning,
            "metaDL" or "forcedMetaDL" => dhtEnabled ? DownloadItemStatus.Queued : DownloadItemStatus.Warning,
            "forcedDL" or "moving" or "downloading" => DownloadItemStatus.Downloading,
            _ => DownloadItemStatus.Downloading
        };
    }
}

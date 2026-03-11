// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Download.Clients.Transmission;

public class TransmissionClient : IDownloadClient
{
    private readonly TransmissionProxy _proxy;
    private readonly TransmissionSettings _settings;
    private readonly ILogger<TransmissionClient> _logger;

    public TransmissionClient(
        TransmissionProxy proxy,
        TransmissionSettings settings,
        ILogger<TransmissionClient> logger)
    {
        _proxy = proxy;
        _settings = settings;
        _logger = logger;
    }

    public string Name => "Transmission";
    public DownloadProtocol Protocol => DownloadProtocol.Torrent;

    public async Task<bool> TestConnectionAsync(CancellationToken cancellationToken = default)
    {
        return await _proxy.TestConnectionAsync(_settings, cancellationToken);
    }

    public async Task<IEnumerable<DownloadClientItem>> GetItemsAsync(CancellationToken cancellationToken = default)
    {
        var torrents = await _proxy.GetTorrentsAsync(_settings, cancellationToken);

        // Filter by category label if configured
        if (!string.IsNullOrWhiteSpace(_settings.Category))
        {
            torrents = torrents
                .Where(t => t.Labels.Contains(_settings.Category, StringComparer.OrdinalIgnoreCase))
                .ToList();
        }

        return torrents.Select(t => new DownloadClientItem
        {
            DownloadId = t.HashString.ToUpperInvariant(),
            Category = t.Labels.FirstOrDefault() ?? string.Empty,
            Title = t.Name,
            TotalSize = t.TotalSize,
            RemainingSize = t.LeftUntilDone,
            RemainingTime = GetRemainingTime(t),
            SeedRatio = t.UploadRatio,
            OutputPath = t.DownloadDir,
            Message = t.ErrorString,
            Status = MapStatus(t),
            CanMoveFiles = false,
            CanBeRemoved = t.IsFinished || t.Status == TransmissionTorrentStatus.Stopped
        });
    }

    public async Task<DownloadClientInfo> GetStatusAsync(CancellationToken cancellationToken = default)
    {
        var session = await _proxy.GetSessionInfoAsync(_settings, cancellationToken);

        return new DownloadClientInfo
        {
            IsLocalhost = _settings.Host is "127.0.0.1" or "localhost",
            OutputRootFolders = new List<string>
            {
                !string.IsNullOrWhiteSpace(_settings.DownloadDirectory)
                    ? _settings.DownloadDirectory
                    : session.DownloadDir
            },
            RemovesCompletedDownloads = false
        };
    }

    public async Task RemoveItemAsync(string downloadId, bool deleteData, CancellationToken cancellationToken = default)
    {
        await _proxy.RemoveTorrentAsync(downloadId, deleteData, _settings, cancellationToken);
        _logger.LogInformation("Removed torrent {DownloadId} from Transmission (deleteData: {DeleteData})",
            downloadId, deleteData);
    }

    private static TimeSpan? GetRemainingTime(TransmissionTorrent torrent)
    {
        if (torrent.Eta < 0 || torrent.Eta > 365 * 24 * 3600)
        {
            return null;
        }

        return TimeSpan.FromSeconds(torrent.Eta);
    }

    private static DownloadItemStatus MapStatus(TransmissionTorrent torrent)
    {
        if (torrent.Error > 0)
        {
            return DownloadItemStatus.Warning;
        }

        return torrent.Status switch
        {
            TransmissionTorrentStatus.Stopped => torrent.IsFinished
                ? DownloadItemStatus.Completed
                : DownloadItemStatus.Paused,
            TransmissionTorrentStatus.CheckPending or
            TransmissionTorrentStatus.Checking => DownloadItemStatus.Queued,
            TransmissionTorrentStatus.DownloadPending => DownloadItemStatus.Queued,
            TransmissionTorrentStatus.Downloading => DownloadItemStatus.Downloading,
            TransmissionTorrentStatus.SeedPending or
            TransmissionTorrentStatus.Seeding => DownloadItemStatus.Completed,
            _ => DownloadItemStatus.Downloading
        };
    }
}

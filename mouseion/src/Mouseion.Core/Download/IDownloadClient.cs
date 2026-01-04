// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Download;

/// <summary>
/// Download client interface for torrent and usenet clients
/// </summary>
public interface IDownloadClient
{
    /// <summary>
    /// Unique name for this download client implementation
    /// </summary>
    string Name { get; }

    /// <summary>
    /// Protocol supported by this client (Torrent or Usenet)
    /// </summary>
    DownloadProtocol Protocol { get; }

    /// <summary>
    /// Test connection to the download client
    /// </summary>
    Task<bool> TestConnectionAsync(CancellationToken cancellationToken = default);

    /// <summary>
    /// Get all items from the download client
    /// </summary>
    Task<IEnumerable<DownloadClientItem>> GetItemsAsync(CancellationToken cancellationToken = default);

    /// <summary>
    /// Get status information from the download client
    /// </summary>
    Task<DownloadClientInfo> GetStatusAsync(CancellationToken cancellationToken = default);

    /// <summary>
    /// Remove an item from the download client
    /// </summary>
    Task RemoveItemAsync(string downloadId, bool deleteData, CancellationToken cancellationToken = default);
}

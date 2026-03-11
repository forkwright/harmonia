// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Download.Clients.QBittorrent;

public class QBittorrentSettings
{
    public string Host { get; set; } = "localhost";
    public int Port { get; set; } = 8080;
    public bool UseSsl { get; set; }
    public string UrlBase { get; set; } = string.Empty;
    public string Username { get; set; } = string.Empty;
    public string Password { get; set; } = string.Empty;
    public string Category { get; set; } = "mouseion";
    public string PostImportCategory { get; set; } = string.Empty;
}

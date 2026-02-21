// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Download.Clients.Deluge;

public class DelugeSettings
{
    public string Host { get; set; } = "localhost";
    public int Port { get; set; } = 8112;
    public bool UseSsl { get; set; }
    public string UrlBase { get; set; } = string.Empty;
    public string Password { get; set; } = "deluge";
    public string Category { get; set; } = "mouseion";
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Download.Clients.SABnzbd;

public class SABnzbdSettings
{
    public string Host { get; set; } = "localhost";
    public int Port { get; set; } = 8080;
    public bool UseSsl { get; set; }
    public string UrlBase { get; set; } = string.Empty;
    public string ApiKey { get; set; } = string.Empty;
    public string Category { get; set; } = "mouseion";
}

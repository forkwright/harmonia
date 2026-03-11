// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Download.Clients.Transmission;

public class TransmissionSettings
{
    public string Host { get; set; } = "localhost";
    public int Port { get; set; } = 9091;
    public bool UseSsl { get; set; }
    public string UrlBase { get; set; } = "/transmission/rpc";
    public string Username { get; set; } = string.Empty;
    public string Password { get; set; } = string.Empty;
    public string DownloadDirectory { get; set; } = string.Empty;
    public string Category { get; set; } = "mouseion";
}

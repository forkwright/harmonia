// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Common.Disk;
using Mouseion.Common.Http;

namespace Mouseion.Core.MediaCovers;

public interface ICoverExistsSpecification
{
    bool AlreadyExists(string url, string path);
}

public class CoverExistsSpecification : ICoverExistsSpecification
{
    private readonly IDiskProvider _diskProvider;
    private readonly IHttpClient _httpClient;
    private readonly ILogger<CoverExistsSpecification> _logger;

    public CoverExistsSpecification(IDiskProvider diskProvider, IHttpClient httpClient, ILogger<CoverExistsSpecification> logger)
    {
        _diskProvider = diskProvider;
        _httpClient = httpClient;
        _logger = logger;
    }

    public bool AlreadyExists(string url, string path)
    {
        if (!_diskProvider.FileExists(path))
        {
            return false;
        }

        try
        {
            var headers = _httpClient.Head(new HttpRequest(url)).Headers;
            var fileSize = _diskProvider.GetFileSize(path);
            return fileSize == headers.ContentLength;
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Failed to check if cover already exists for {Url}", url);
            return true; // Assume exists to avoid re-download on temporary failures
        }
    }
}

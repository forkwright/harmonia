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
    Task<bool> AlreadyExistsAsync(string url, string path);
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

    public async Task<bool> AlreadyExistsAsync(string url, string path)
    {
        if (!_diskProvider.FileExists(path))
        {
            return false;
        }

        try
        {
            var response = await _httpClient.HeadAsync(new HttpRequest(url)).ConfigureAwait(false);
            var fileSize = _diskProvider.GetFileSize(path);
            return fileSize == response.Headers.ContentLength;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogWarning(ex, "Network error checking if cover exists for {Url}", url);
            return true; // Assume exists to avoid re-download on temporary failures
        }
        catch (HttpException ex)
        {
            _logger.LogWarning(ex, "HTTP error checking if cover exists for {Url}", url);
            return true; // Assume exists to avoid re-download on temporary failures
        }
        catch (IOException ex)
        {
            _logger.LogWarning(ex, "I/O error checking if cover exists for {Url}", url);
            return true; // Assume exists to avoid re-download on temporary failures
        }
    }
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;

namespace Mouseion.Core.MediaFiles.MediaInfo;

public interface IUpdateMediaInfoService
{
    bool Update(MediaFile mediaFile, string mediaItemPath);
}

public class UpdateMediaInfoService : IUpdateMediaInfoService
{
    private readonly IMediaInfoService _mediaInfoService;
    private readonly IMediaFileRepository _mediaFileRepository;
    private readonly ILogger<UpdateMediaInfoService> _logger;

    public UpdateMediaInfoService(
        IMediaInfoService mediaInfoService,
        IMediaFileRepository mediaFileRepository,
        ILogger<UpdateMediaInfoService> logger)
    {
        _mediaInfoService = mediaInfoService;
        _mediaFileRepository = mediaFileRepository;
        _logger = logger;
    }

    public bool Update(MediaFile mediaFile, string mediaItemPath)
    {
        string path;
        if (!string.IsNullOrWhiteSpace(mediaFile.Path))
        {
            path = mediaFile.Path;
        }
        else if (!string.IsNullOrWhiteSpace(mediaFile.RelativePath))
        {
            if (!mediaItemPath.IsPathTraversalSafe(mediaFile.RelativePath))
            {
                _logger.LogWarning("Rejecting path traversal attempt: {BasePath} + {RelativePath}", mediaItemPath, mediaFile.RelativePath);
                return false;
            }

            // Path traversal safe: validated by IsPathTraversalSafe() on line 45
            path = Path.Combine(mediaItemPath, mediaFile.RelativePath);
        }
        else
        {
            _logger.LogDebug("MediaFile has neither Path nor RelativePath set");
            return false;
        }

        if (!File.Exists(path))
        {
            _logger.LogDebug("Can't update MediaInfo because '{Path}' does not exist", path);
            return false;
        }

        var updatedMediaInfo = _mediaInfoService.GetMediaInfo(path);

        if (updatedMediaInfo == null)
        {
            return false;
        }

        mediaFile.MediaInfo = updatedMediaInfo;
        _mediaFileRepository.Update(mediaFile);
        _logger.LogDebug("Updated MediaInfo for '{Path}'", path);

        return true;
    }
}

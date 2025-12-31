// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Common.EnvironmentInfo;
using Mouseion.Common.Security;
using Mouseion.Core.MediaFiles;

namespace Mouseion.Api.Chapters;

[ApiController]
[Route("api/v3/chapters")]
[Authorize]
public class ChaptersController : ControllerBase
{
    private readonly IMediaFileRepository _mediaFileRepository;
    private readonly IMediaAnalyzer _mediaAnalyzer;
    private readonly IPathValidator _pathValidator;
    private readonly IAppFolderInfo _appFolderInfo;

    public ChaptersController(
        IMediaFileRepository mediaFileRepository,
        IMediaAnalyzer mediaAnalyzer,
        IPathValidator pathValidator,
        IAppFolderInfo appFolderInfo)
    {
        _mediaFileRepository = mediaFileRepository;
        _mediaAnalyzer = mediaAnalyzer;
        _pathValidator = pathValidator;
        _appFolderInfo = appFolderInfo;
    }

    [HttpGet("{mediaFileId:int}")]
    public ActionResult<List<ChapterInfo>> GetChapters(int mediaFileId)
    {
        var mediaFile = _mediaFileRepository.Find(mediaFileId);
        if (mediaFile == null)
        {
            return NotFound(new { error = $"MediaFile {mediaFileId} not found" });
        }

        // Validate path to prevent path traversal
        try
        {
            var validatedPath = _pathValidator.ValidateAndNormalizePath(
                mediaFile.Path,
                _appFolderInfo.AppDataFolder);

            if (!System.IO.File.Exists(validatedPath))
            {
                return NotFound(new { error = "File not found" });
            }

            var chapters = _mediaAnalyzer.GetChapters(validatedPath);
            return Ok(chapters);
        }
        catch (System.Security.SecurityException)
        {
            return Forbid();
        }
    }
}

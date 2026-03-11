// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.Music;

namespace Mouseion.Core.MediaFiles.Import.Specifications;

public class AlreadyImportedSpecification : IImportSpecification
{
    private readonly IMusicFileRepository _musicFileRepository;
    private readonly ILogger<AlreadyImportedSpecification> _logger;

    public AlreadyImportedSpecification(
        IMusicFileRepository musicFileRepository,
        ILogger<AlreadyImportedSpecification> logger)
    {
        _musicFileRepository = musicFileRepository;
        _logger = logger;
    }

    public async Task<ImportRejection?> IsSatisfiedByAsync(MusicFileInfo musicFileInfo, CancellationToken ct = default)
    {
        var relativePath = GetRelativePath(musicFileInfo.Path);

        var existingFile = await _musicFileRepository.FindByPathAsync(relativePath, ct).ConfigureAwait(false);

        if (existingFile != null && existingFile.Size == musicFileInfo.Size)
        {
            _logger.LogDebug("File already imported: {Path} (Size: {Size})", musicFileInfo.Path, musicFileInfo.Size);
            return new ImportRejection(
                ImportRejectionReason.AlreadyImported,
                $"File already imported: {relativePath}");
        }

        return null;
    }

    public ImportRejection? IsSatisfiedBy(MusicFileInfo musicFileInfo)
    {
        var relativePath = GetRelativePath(musicFileInfo.Path);

        var existingFile = _musicFileRepository.FindByPath(relativePath);

        if (existingFile != null && existingFile.Size == musicFileInfo.Size)
        {
            _logger.LogDebug("File already imported: {Path} (Size: {Size})", musicFileInfo.Path, musicFileInfo.Size);
            return new ImportRejection(
                ImportRejectionReason.AlreadyImported,
                $"File already imported: {relativePath}");
        }

        return null;
    }

    private static string GetRelativePath(string fullPath)
    {
        return fullPath;
    }
}

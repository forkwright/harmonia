// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.Music;
using Mouseion.Core.Qualities;

namespace Mouseion.Core.MediaFiles.Import;

public interface IImportApprovedFiles
{
    Task<List<MusicFile>> ImportAsync(List<ImportDecision> decisions, CancellationToken ct = default);
}

public class ImportApprovedFiles : IImportApprovedFiles
{
    private readonly IMusicFileRepository _musicFileRepository;
    private readonly ILogger<ImportApprovedFiles> _logger;

    public ImportApprovedFiles(
        IMusicFileRepository musicFileRepository,
        ILogger<ImportApprovedFiles> logger)
    {
        _musicFileRepository = musicFileRepository;
        _logger = logger;
    }

    public async Task<List<MusicFile>> ImportAsync(List<ImportDecision> decisions, CancellationToken ct = default)
    {
        var approvedDecisions = decisions.Where(d => d.Approved).ToList();

        if (approvedDecisions.Count == 0)
        {
            _logger.LogInformation("No files approved for import");
            return new List<MusicFile>();
        }

        _logger.LogInformation("Importing {Count} approved files", approvedDecisions.Count);

        var importTasks = approvedDecisions.Select(d => ImportFileAsync(d.MusicFileInfo, ct));
        var musicFiles = await Task.WhenAll(importTasks).ConfigureAwait(false);
        var importedFiles = musicFiles.OfType<MusicFile>().ToList();

        _logger.LogInformation("Successfully imported {Count} files", importedFiles.Count);

        return importedFiles;
    }

    private async Task<MusicFile?> ImportFileAsync(MusicFileInfo musicFileInfo, CancellationToken ct)
    {
        try
        {
            var musicFile = new MusicFile
            {
                RelativePath = musicFileInfo.Path,
                Size = musicFileInfo.Size,
                DateAdded = DateTime.UtcNow,
                Quality = new QualityModel { Quality = musicFileInfo.Quality },
                AudioFormat = musicFileInfo.Codec,
                Bitrate = musicFileInfo.Bitrate,
                SampleRate = musicFileInfo.SampleRate,
                Channels = musicFileInfo.Channels
            };

            var inserted = await _musicFileRepository.InsertAsync(musicFile, ct).ConfigureAwait(false);
            _logger.LogDebug("Imported file: {Path} (ID: {Id})", musicFileInfo.Path, inserted.Id);

            return inserted;
        }
        catch (InvalidOperationException ex)
        {
            _logger.LogError(ex, "Failed to import file: {Path}", musicFileInfo.Path);
            return null;
        }
        catch (IOException ex)
        {
            _logger.LogError(ex, "Failed to import file: {Path}", musicFileInfo.Path);
            return null;
        }
    }
}

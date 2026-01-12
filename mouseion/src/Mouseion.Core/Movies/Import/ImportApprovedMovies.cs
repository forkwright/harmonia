// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.MediaFiles.Import;
using Mouseion.Core.Movies;
using Mouseion.Core.Movies.Organization;

namespace Mouseion.Core.Movies.Import;

public class ImportApprovedMovies : IImportApprovedMovies
{
    private readonly IMovieFileRepository _movieFileRepository;
    private readonly IFileImportService _fileImportService;
    private readonly ILogger<ImportApprovedMovies> _logger;

    public ImportApprovedMovies(
        IMovieFileRepository movieFileRepository,
        IFileImportService fileImportService,
        ILogger<ImportApprovedMovies> logger)
    {
        _movieFileRepository = movieFileRepository;
        _fileImportService = fileImportService;
        _logger = logger;
    }

    public async Task<List<MovieImportResult>> ImportAsync(List<MovieImportDecision> decisions, CancellationToken ct = default)
    {
        var results = new List<MovieImportResult>();

        var approvedDecisions = decisions.Where(d => d.Approved).ToList();
        _logger.LogInformation("Importing {Count} approved movie files", approvedDecisions.Count);

        foreach (var decision in approvedDecisions)
        {
            try
            {
                // Construct destination path in movie directory
                var sourceFileName = System.IO.Path.GetFileName(decision.FilePath);
                var destinationPath = System.IO.Path.Combine(decision.Movie.Path, sourceFileName);

                _logger.LogDebug("Importing {Source} → {Destination}", decision.FilePath, destinationPath);

                // Physically import the file using FileImportService
                var importResult = await _fileImportService.ImportFileAsync(
                    decision.FilePath,
                    destinationPath,
                    preferredStrategy: null, // Auto-select strategy
                    verifyChecksum: true);

                if (!importResult.IsSuccess)
                {
                    _logger.LogError("File import failed: {Error}", importResult.ErrorMessage);
                    results.Add(new MovieImportResult(decision, success: false, errorMessage: importResult.ErrorMessage));
                    continue;
                }

                // Create database record with imported file path
                var movieFile = new MovieFile
                {
                    MovieId = decision.Movie.Id,
                    Path = importResult.DestinationPath!,
                    DateAdded = DateTime.UtcNow
                };

                await _movieFileRepository.InsertAsync(movieFile, ct).ConfigureAwait(false);

                _logger.LogInformation("Imported movie file: {FilePath} for {Title} ({Year}) using {Strategy}",
                    importResult.DestinationPath, decision.Movie.Title, decision.Movie.Year, importResult.RequestedStrategy);

                results.Add(new MovieImportResult(decision, success: true));
            }
            catch (IOException ex)
            {
                _logger.LogError(ex, "I/O error importing movie file: {FilePath}", decision.FilePath);
                results.Add(new MovieImportResult(decision, success: false, errorMessage: ex.Message));
            }
            catch (InvalidOperationException ex)
            {
                _logger.LogError(ex, "Database error importing movie file: {FilePath}", decision.FilePath);
                results.Add(new MovieImportResult(decision, success: false, errorMessage: ex.Message));
            }
            catch (UnauthorizedAccessException ex)
            {
                _logger.LogError(ex, "Access denied importing movie file: {FilePath}", decision.FilePath);
                results.Add(new MovieImportResult(decision, success: false, errorMessage: ex.Message));
            }
        }

        var rejectedDecisions = decisions.Where(d => !d.Approved).ToList();
        foreach (var decision in rejectedDecisions)
        {
            var reasons = string.Join(", ", decision.Rejections.Select(r => r.Message));
            results.Add(new MovieImportResult(decision, success: false, errorMessage: $"Rejected: {reasons}"));
        }

        return results;
    }
}

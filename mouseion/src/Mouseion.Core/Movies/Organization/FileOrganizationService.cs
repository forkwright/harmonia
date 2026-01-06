// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.RegularExpressions;
using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;
using Mouseion.Core.MediaFiles;

namespace Mouseion.Core.Movies.Organization;

public interface IFileOrganizationService
{
    Task<OrganizationResult> PreviewRenameAsync(int movieId, string namingPattern, CancellationToken ct = default);
    Task<OrganizationResult> RenameMovieAsync(int movieId, string namingPattern, FileStrategy strategy = FileStrategy.Hardlink, CancellationToken ct = default);

    OrganizationResult PreviewRename(int movieId, string namingPattern);
    OrganizationResult RenameMovie(int movieId, string namingPattern, FileStrategy strategy = FileStrategy.Hardlink);
}

public class FileOrganizationService : IFileOrganizationService
{
    private readonly IMovieRepository _movieRepository;
    private readonly IMediaFileRepository _mediaFileRepository;
    private readonly ILogger<FileOrganizationService> _logger;

    private static readonly Regex TokenRegex = new Regex(@"\{(?<token>[^}]+)\}", RegexOptions.Compiled, TimeSpan.FromSeconds(1));

    public FileOrganizationService(
        IMovieRepository movieRepository,
        IMediaFileRepository mediaFileRepository,
        ILogger<FileOrganizationService> logger)
    {
        _movieRepository = movieRepository;
        _mediaFileRepository = mediaFileRepository;
        _logger = logger;
    }

    public async Task<OrganizationResult> PreviewRenameAsync(int movieId, string namingPattern, CancellationToken ct = default)
    {
        return await ExecuteRenameAsync(movieId, namingPattern, FileStrategy.Hardlink, dryRun: true, ct).ConfigureAwait(false);
    }

    public async Task<OrganizationResult> RenameMovieAsync(int movieId, string namingPattern, FileStrategy strategy = FileStrategy.Hardlink, CancellationToken ct = default)
    {
        return await ExecuteRenameAsync(movieId, namingPattern, strategy, dryRun: false, ct).ConfigureAwait(false);
    }

    public OrganizationResult PreviewRename(int movieId, string namingPattern)
    {
        return ExecuteRename(movieId, namingPattern, FileStrategy.Hardlink, dryRun: true);
    }

    public OrganizationResult RenameMovie(int movieId, string namingPattern, FileStrategy strategy = FileStrategy.Hardlink)
    {
        return ExecuteRename(movieId, namingPattern, strategy, dryRun: false);
    }

    private async Task<OrganizationResult> ExecuteRenameAsync(int movieId, string namingPattern, FileStrategy strategy, bool dryRun, CancellationToken ct)
    {
        var movie = await _movieRepository.FindAsync(movieId, ct).ConfigureAwait(false);
        if (movie == null)
        {
            return new OrganizationResult
            {
                Success = false,
                ErrorMessage = $"Movie with ID {movieId} not found"
            };
        }

        var mediaFiles = await _mediaFileRepository.GetByMediaItemIdAsync(movieId, ct).ConfigureAwait(false);
        if (mediaFiles == null || mediaFiles.Count == 0)
        {
            return new OrganizationResult
            {
                Success = false,
                ErrorMessage = $"No media files found for movie: {movie.Title.SanitizeForLog()} ({movie.Year})"
            };
        }

        var primaryFile = mediaFiles.OrderByDescending(f => f.Size).First();
        var originalPath = primaryFile.Path;

        if (!File.Exists(originalPath))
        {
            return new OrganizationResult
            {
                Success = false,
                OriginalPath = originalPath,
                ErrorMessage = "Original file does not exist"
            };
        }

        var newFileName = ParseNamingPattern(namingPattern, movie, primaryFile);
        var movieFolder = Path.Combine(movie.RootFolderPath, SanitizeFileName(movie.Title));
        var newPath = Path.Combine(movieFolder, newFileName);

        if (dryRun)
        {
            _logger.LogInformation("[DRY RUN] Would rename: {OriginalPath} -> {NewPath}",
                originalPath.SanitizeForLog(), newPath.SanitizeForLog());

            return new OrganizationResult
            {
                Success = true,
                OriginalPath = originalPath,
                NewPath = newPath,
                StrategyUsed = strategy,
                IsDryRun = true
            };
        }

        try
        {
            Directory.CreateDirectory(movieFolder);

            switch (strategy)
            {
                case FileStrategy.Hardlink:
                    await CreateHardlinkAsync(originalPath, newPath, ct).ConfigureAwait(false);
                    break;
                case FileStrategy.Copy:
                    File.Copy(originalPath, newPath, overwrite: false);
                    break;
                case FileStrategy.Move:
                    File.Move(originalPath, newPath, overwrite: false);
                    break;
                case FileStrategy.Symlink:
                    CreateSymlink(newPath, originalPath);
                    break;
            }

            primaryFile.Path = newPath;
            primaryFile.RelativePath = Path.GetFileName(newPath);
            await _mediaFileRepository.UpdateAsync(primaryFile, ct).ConfigureAwait(false);

            _logger.LogInformation("Renamed movie file: {OriginalPath} -> {NewPath} (strategy: {Strategy})",
                originalPath.SanitizeForLog(), newPath.SanitizeForLog(), strategy);

            return new OrganizationResult
            {
                Success = true,
                OriginalPath = originalPath,
                NewPath = newPath,
                StrategyUsed = strategy,
                IsDryRun = false
            };
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to rename movie file: {OriginalPath}", originalPath.SanitizeForLog());

            return new OrganizationResult
            {
                Success = false,
                OriginalPath = originalPath,
                ErrorMessage = ex.Message,
                StrategyUsed = strategy
            };
        }
    }

    private OrganizationResult ExecuteRename(int movieId, string namingPattern, FileStrategy strategy, bool dryRun)
    {
        var movie = _movieRepository.Find(movieId);
        if (movie == null)
        {
            return new OrganizationResult
            {
                Success = false,
                ErrorMessage = $"Movie with ID {movieId} not found"
            };
        }

        var mediaFiles = _mediaFileRepository.GetByMediaItemId(movieId).ToList();
        if (mediaFiles == null || mediaFiles.Count == 0)
        {
            return new OrganizationResult
            {
                Success = false,
                ErrorMessage = $"No media files found for movie: {movie.Title.SanitizeForLog()} ({movie.Year})"
            };
        }

        var primaryFile = mediaFiles.OrderByDescending(f => f.Size).First();
        var originalPath = primaryFile.Path;

        if (!File.Exists(originalPath))
        {
            return new OrganizationResult
            {
                Success = false,
                OriginalPath = originalPath,
                ErrorMessage = "Original file does not exist"
            };
        }

        var newFileName = ParseNamingPattern(namingPattern, movie, primaryFile);
        var movieFolder = Path.Combine(movie.RootFolderPath, SanitizeFileName(movie.Title));
        var newPath = Path.Combine(movieFolder, newFileName);

        if (dryRun)
        {
            _logger.LogInformation("[DRY RUN] Would rename: {OriginalPath} -> {NewPath}",
                originalPath.SanitizeForLog(), newPath.SanitizeForLog());

            return new OrganizationResult
            {
                Success = true,
                OriginalPath = originalPath,
                NewPath = newPath,
                StrategyUsed = strategy,
                IsDryRun = true
            };
        }

        try
        {
            Directory.CreateDirectory(movieFolder);

            switch (strategy)
            {
                case FileStrategy.Hardlink:
                    CreateHardlink(originalPath, newPath);
                    break;
                case FileStrategy.Copy:
                    File.Copy(originalPath, newPath, overwrite: false);
                    break;
                case FileStrategy.Move:
                    File.Move(originalPath, newPath, overwrite: false);
                    break;
                case FileStrategy.Symlink:
                    CreateSymlink(newPath, originalPath);
                    break;
            }

            primaryFile.Path = newPath;
            primaryFile.RelativePath = Path.GetFileName(newPath);
            _mediaFileRepository.Update(primaryFile);

            _logger.LogInformation("Renamed movie file: {OriginalPath} -> {NewPath} (strategy: {Strategy})",
                originalPath.SanitizeForLog(), newPath.SanitizeForLog(), strategy);

            return new OrganizationResult
            {
                Success = true,
                OriginalPath = originalPath,
                NewPath = newPath,
                StrategyUsed = strategy,
                IsDryRun = false
            };
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to rename movie file: {OriginalPath}", originalPath.SanitizeForLog());

            return new OrganizationResult
            {
                Success = false,
                OriginalPath = originalPath,
                ErrorMessage = ex.Message,
                StrategyUsed = strategy
            };
        }
    }

    private string ParseNamingPattern(string pattern, Movie movie, MediaFile mediaFile)
    {
        var result = TokenRegex.Replace(pattern, match =>
        {
            var token = match.Groups["token"].Value.Trim().ToLowerInvariant();

            return token switch
            {
                "movie title" or "title" => SanitizeFileName(movie.Title),
                "movie year" or "year" => movie.Year.ToString(),
                "quality" => SanitizeFileName(mediaFile.Quality ?? "Unknown"),
                "studio" => SanitizeFileName(movie.Studio ?? "Unknown"),
                "certification" => SanitizeFileName(movie.Certification ?? "Unrated"),
                "tmdbid" => movie.TmdbId ?? "unknown",
                "imdbid" => movie.ImdbId ?? "unknown",
                "file extension" or "extension" => Path.GetExtension(mediaFile.Path),
                _ => match.Value
            };
        });

        if (!Path.HasExtension(result))
        {
            result += Path.GetExtension(mediaFile.Path);
        }

        return result;
    }

    private static string SanitizeFileName(string fileName)
    {
        var invalid = Path.GetInvalidFileNameChars();
        var sanitized = string.Join("_", fileName.Split(invalid, StringSplitOptions.RemoveEmptyEntries));
        return sanitized.Trim();
    }

    private async Task CreateHardlinkAsync(string sourcePath, string targetPath, CancellationToken ct)
    {
        await Task.Run(() => CreateHardlink(sourcePath, targetPath), ct).ConfigureAwait(false);
    }

    private static void CreateHardlink(string sourcePath, string targetPath)
    {
        if (OperatingSystem.IsWindows())
        {
            if (!NativeMethods.CreateHardLink(targetPath, sourcePath, IntPtr.Zero))
            {
                throw new IOException($"Failed to create hardlink: {targetPath} -> {sourcePath}");
            }
        }
        else
        {
            var link = System.Runtime.InteropServices.NativeLibrary.Load("c");
            var linkMethod = System.Runtime.InteropServices.Marshal.GetDelegateForFunctionPointer<LinkDelegate>(
                System.Runtime.InteropServices.NativeLibrary.GetExport(link, "link"));

            if (linkMethod(sourcePath, targetPath) != 0)
            {
                throw new IOException($"Failed to create hardlink: {targetPath} -> {sourcePath}");
            }
        }
    }

    private static void CreateSymlink(string linkPath, string targetPath)
    {
        if (OperatingSystem.IsWindows())
        {
            if (!NativeMethods.CreateSymbolicLink(linkPath, targetPath, 0))
            {
                throw new IOException($"Failed to create symlink: {linkPath} -> {targetPath}");
            }
        }
        else
        {
            File.CreateSymbolicLink(linkPath, targetPath);
        }
    }

    [System.Runtime.InteropServices.UnmanagedFunctionPointer(System.Runtime.InteropServices.CallingConvention.Cdecl)]
    private delegate int LinkDelegate(string oldPath, string newPath);

    private static class NativeMethods
    {
        [System.Runtime.InteropServices.DllImport("kernel32.dll", SetLastError = true, CharSet = System.Runtime.InteropServices.CharSet.Unicode)]
        [return: System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.Bool)]
        public static extern bool CreateHardLink(string lpFileName, string lpExistingFileName, IntPtr lpSecurityAttributes);

        [System.Runtime.InteropServices.DllImport("kernel32.dll", SetLastError = true, CharSet = System.Runtime.InteropServices.CharSet.Unicode)]
        [return: System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.Bool)]
        public static extern bool CreateSymbolicLink(string lpSymlinkFileName, string lpTargetFileName, int dwFlags);
    }
}

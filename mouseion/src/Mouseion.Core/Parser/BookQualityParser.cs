// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.RegularExpressions;
using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;
using Mouseion.Core.MediaFiles;
using Mouseion.Core.Qualities;

namespace Mouseion.Core.Parser;

public static class BookQualityParser
{
    private static readonly TimeSpan RegexTimeout = TimeSpan.FromSeconds(5);

    private static readonly Regex FormatRegex = new(
        @"\b(?:
            (?<epub>EPUB|ePub)|
            (?<azw3>AZW3?|Kindle[-_.\s]?Format[-_.\s]?8|KF8)|
            (?<mobi>MOBI|Kindle)|
            (?<pdf>PDF)|
            (?<txt>TXT|Plain[-_.\s]?Text)|
            (?<cbr>CBR|Comic[-_.\s]?Book[-_.\s]?RAR)|
            (?<cbz>CBZ|Comic[-_.\s]?Book[-_.\s]?ZIP)
        )\b",
        RegexOptions.Compiled | RegexOptions.IgnoreCase | RegexOptions.IgnorePatternWhitespace,
        RegexTimeout);

    public static QualityModel ParseQuality(string name, ILogger? logger = null)
    {
        logger?.LogDebug("Trying to parse book quality for '{Name}'", name.SanitizeForLog());

        if (name.IsNullOrWhiteSpace())
        {
            return new QualityModel { Quality = Quality.EbookUnknown };
        }

        name = name.Trim();

        // Try extension-based detection first for files with ebook extensions
        if (!name.ContainsInvalidPathChars())
        {
            var extension = Path.GetExtension(name);
            if (!string.IsNullOrEmpty(extension) && MediaFileExtensions.EbookExtensions.Contains(extension))
            {
                // Strip extension before parsing name to avoid false matches (e.g., ".epub" matching "epub" format)
                var nameWithoutExtension = Path.GetFileNameWithoutExtension(name);
                var normalizedName = nameWithoutExtension.Replace('_', ' ').Trim();
                var result = ParseQualityName(normalizedName);

                // If name parsing found quality, use it (e.g., "Author - Title [PDF].epub")
                // Otherwise, use extension-based default
                if (result.Quality != Quality.EbookUnknown)
                {
                    return result;
                }

                result.Quality = MediaFileExtensions.GetQualityForExtension(extension);
                result.SourceDetectionSource = QualityDetectionSource.Extension;
                return result;
            }
        }

        // Fall back to name-based parsing (for non-file inputs or files without extensions)
        var normalizedNameFallback = name.Replace('_', ' ').Trim();
        return ParseQualityName(normalizedNameFallback);
    }

    private static QualityModel ParseFromExtension(string name, QualityModel result, ILogger? logger = null)
    {
        try
        {
            var extension = Path.GetExtension(name);
            if (string.IsNullOrEmpty(extension))
            {
                return result;
            }

            if (MediaFileExtensions.EbookExtensions.Contains(extension))
            {
                result.Quality = MediaFileExtensions.GetQualityForExtension(extension);
                result.SourceDetectionSource = QualityDetectionSource.Extension;
            }
        }
        catch (ArgumentException ex)
        {
            logger?.LogDebug(ex, "Unable to parse extension from '{Name}'", name.SanitizeForLog());
        }

        return result;
    }

    private static QualityModel ParseQualityName(string name)
    {
        var result = new QualityModel { Quality = Quality.EbookUnknown };

        var formatMatch = FormatRegex.Match(name);

        if (!formatMatch.Success)
        {
            return result;
        }

        result.SourceDetectionSource = QualityDetectionSource.Name;

        if (formatMatch.Groups["epub"].Success)
        {
            result.Quality = Quality.EPUB;
            return result;
        }

        // Check azw3 before mobi to handle "Kindle Format 8" correctly
        if (formatMatch.Groups["azw3"].Success)
        {
            result.Quality = Quality.AZW3;
            return result;
        }

        if (formatMatch.Groups["mobi"].Success)
        {
            result.Quality = Quality.MOBI;
            return result;
        }

        if (formatMatch.Groups["pdf"].Success)
        {
            result.Quality = Quality.PDF;
            return result;
        }

        if (formatMatch.Groups["txt"].Success)
        {
            result.Quality = Quality.TXT;
            return result;
        }

        if (formatMatch.Groups["cbr"].Success || formatMatch.Groups["cbz"].Success)
        {
            result.Quality = Quality.EbookUnknown;
            return result;
        }

        return result;
    }

    public static bool IsBookFile(string path)
    {
        if (string.IsNullOrWhiteSpace(path))
        {
            return false;
        }

        try
        {
            var extension = Path.GetExtension(path);
            return MediaFileExtensions.EbookExtensions.Contains(extension);
        }
        catch
        {
            return false;
        }
    }
}

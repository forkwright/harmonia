// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.RegularExpressions;
using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;
using Mouseion.Core.MediaFiles;
using Mouseion.Core.Qualities;

namespace Mouseion.Core.Parser;

public static class AudiobookQualityParser
{
    private static readonly TimeSpan RegexTimeout = TimeSpan.FromSeconds(5);

    private static readonly Regex BitrateRegex = new(
        @"\b(?:
            (?<mp3_128>MP3[-_. ]?128|128[-_. ]?kbps?|128k(?:bit)?)|
            (?<mp3_320>MP3[-_. ]?320|320[-_. ]?kbps?|320k(?:bit)?|MP3[-_. ]?(?:V0|CBR)|V0)|
            (?<vbr>VBR|V[0-9])
        )\b",
        RegexOptions.Compiled | RegexOptions.IgnoreCase | RegexOptions.IgnorePatternWhitespace,
        RegexTimeout);

    private static readonly Regex FormatRegex = new(
        @"\b(?:
            (?<m4b>M4B)|
            (?<mp3>MP3)|
            (?<flac>FLAC)|
            (?<aax>AAX|Audible[-_. ]?Enhanced)|
            (?<aa>AA(?!C)|Audible(?![-_. ]?Enhanced))
        )\b",
        RegexOptions.Compiled | RegexOptions.IgnoreCase | RegexOptions.IgnorePatternWhitespace,
        RegexTimeout);

    private static readonly Regex AudiobookIndicatorRegex = new(
        @"\b(?:
            Audiobook|Audio[-_. ]?Book|Unabridged|Abridged|Narrated[-_. ]?by|Read[-_. ]?by
        )\b",
        RegexOptions.Compiled | RegexOptions.IgnoreCase | RegexOptions.IgnorePatternWhitespace,
        RegexTimeout);

    public static QualityModel ParseQuality(string name, ILogger? logger = null)
    {
        logger?.LogDebug("Trying to parse audiobook quality for '{Name}'", name.SanitizeForLog());

        if (name.IsNullOrWhiteSpace())
        {
            return new QualityModel { Quality = Quality.AudiobookUnknown };
        }

        name = name.Trim();

        // Try extension-based detection first for files with audiobook extensions
        if (!name.ContainsInvalidPathChars())
        {
            var extension = Path.GetExtension(name);
            if (!string.IsNullOrEmpty(extension) && MediaFileExtensions.AudiobookExtensions.Contains(extension))
            {
                // Strip extension before parsing name to avoid false matches (e.g., ".m4b" matching "m4b" format)
                var nameWithoutExtension = Path.GetFileNameWithoutExtension(name);
                var normalizedName = nameWithoutExtension.Replace('_', ' ').Trim();
                var result = ParseQualityName(normalizedName);

                // If name parsing found quality, use it (e.g., "Author - Title [MP3 320].m4b")
                // Otherwise, use extension-based default
                if (result.Quality != Quality.AudiobookUnknown)
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

            if (MediaFileExtensions.AudiobookExtensions.Contains(extension))
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
        var result = new QualityModel { Quality = Quality.AudiobookUnknown };

        var bitrateMatch = BitrateRegex.Match(name);
        var formatMatch = FormatRegex.Match(name);

        if (formatMatch.Success)
        {
            result.SourceDetectionSource = QualityDetectionSource.Name;
            var formatQuality = ParseFormatMatch(formatMatch, bitrateMatch);
            if (formatQuality != Quality.AudiobookUnknown)
            {
                result.Quality = formatQuality;
                return result;
            }
        }

        if (bitrateMatch.Success)
        {
            result.SourceDetectionSource = QualityDetectionSource.Name;
            var bitrateQuality = ParseBitrateMatch(bitrateMatch);
            if (bitrateQuality != Quality.AudiobookUnknown)
            {
                result.Quality = bitrateQuality;
                return result;
            }
        }

        return result;
    }

    private static Quality ParseFormatMatch(Match formatMatch, Match bitrateMatch)
    {
        if (formatMatch.Groups["m4b"].Success || formatMatch.Groups["aax"].Success)
        {
            return Quality.M4B;
        }

        if (formatMatch.Groups["aa"].Success)
        {
            return Quality.MP3_128;
        }

        if (formatMatch.Groups["flac"].Success)
        {
            return Quality.AudioFLAC;
        }

        if (formatMatch.Groups["mp3"].Success)
        {
            return bitrateMatch.Groups["mp3_128"].Success ? Quality.MP3_128 : Quality.MP3_320;
        }

        return Quality.AudiobookUnknown;
    }

    private static Quality ParseBitrateMatch(Match bitrateMatch)
    {
        if (bitrateMatch.Groups["mp3_128"].Success)
        {
            return Quality.MP3_128;
        }

        if (bitrateMatch.Groups["mp3_320"].Success || bitrateMatch.Groups["vbr"].Success)
        {
            return Quality.MP3_320;
        }

        return Quality.AudiobookUnknown;
    }

    public static bool IsAudiobookFile(string path)
    {
        if (string.IsNullOrWhiteSpace(path))
        {
            return false;
        }

        try
        {
            var extension = Path.GetExtension(path);
            return MediaFileExtensions.AudiobookExtensions.Contains(extension);
        }
        catch
        {
            return false;
        }
    }

    public static bool LooksLikeAudiobook(string name)
    {
        return AudiobookIndicatorRegex.IsMatch(name);
    }
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.MediaFiles.Import;

public enum ImportRejectionReason
{
    Unknown,
    FileLocked,
    InvalidFilePath,
    UnsupportedExtension,
    UnableToParse,
    Error,
    AlreadyImported,
    MinimumQuality,
    NoAudioTrack,
    NotQualityUpgrade
}

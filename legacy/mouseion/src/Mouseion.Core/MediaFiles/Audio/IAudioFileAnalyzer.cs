// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.MediaFiles.Audio;

public interface IAudioFileAnalyzer
{
    Task<AudioAnalysisResult> AnalyzeAsync(string filePath, CancellationToken ct = default);
    bool IsFakeHiRes(AudioAnalysisResult analysis);
}

public class AudioAnalysisResult
{
    public int ActualBitDepth { get; set; }
    public int ActualSampleRate { get; set; }
    public int DeclaredBitDepth { get; set; }
    public int DeclaredSampleRate { get; set; }
    public double SpectralCeiling { get; set; }
    public bool HasUltrasonic { get; set; }
    public string Format { get; set; } = null!;
}

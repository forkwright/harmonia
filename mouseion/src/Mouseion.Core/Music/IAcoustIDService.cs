// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Music;

public interface IAcoustIDService
{
    Task<AcoustIDResult?> LookupAsync(string filePath, CancellationToken ct = default);
    Task<string> GenerateFingerprintAsync(string filePath, CancellationToken ct = default);
}

public class AcoustIDResult
{
    public string Fingerprint { get; set; } = null!;
    public int Duration { get; set; }
    public List<AcoustIDRecording> Recordings { get; set; } = new();
}

public class AcoustIDRecording
{
    public string Id { get; set; } = null!;
    public string? Title { get; set; }
    public List<string> Artists { get; set; } = new();
    public string? Album { get; set; }
    public int? Year { get; set; }
    public double Score { get; set; }
}

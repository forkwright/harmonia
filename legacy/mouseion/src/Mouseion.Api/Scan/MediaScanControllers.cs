// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later
// Scan controllers for Movie, TV, Book, and Audiobook media types.
// Mirrors MusicScanController pattern: fire-and-forget for library scans, status polling.

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Hosting;
using Mouseion.Core.MediaFiles;

namespace Mouseion.Api.Scan;

// ─── Movie Scan ────────────────────────────────────────────

[ApiController]
[Route("api/v3/scan/movies")]
[Authorize]
public class MovieScanController : ControllerBase
{
    private readonly IMovieFileScanner _scanner;
    private readonly IHostApplicationLifetime _appLifetime;
    private static ScanStatus? _currentScan;
    private static readonly object _lock = new();

    public MovieScanController(IMovieFileScanner scanner, IHostApplicationLifetime appLifetime)
    {
        _scanner = scanner;
        _appLifetime = appLifetime;
    }

    [HttpGet("status")]
    public ActionResult<ScanStatus> GetStatus()
    {
        lock (_lock) { return Ok(_currentScan ?? new ScanStatus { State = "idle" }); }
    }

    [HttpPost("rootfolder/{id:int}")]
    public ActionResult ScanRootFolder(int id)
    {
        lock (_lock)
        {
            if (_currentScan?.State == "scanning")
                return Conflict(new { error = "Scan already in progress", status = _currentScan });
            _currentScan = new ScanStatus { State = "scanning", StartedAt = DateTime.UtcNow, RootFolderId = id };
        }

        _ = Task.Run(async () =>
        {
            try
            {
                var result = await _scanner.ScanRootFolderAsync(id, _appLifetime.ApplicationStopping);
                lock (_lock)
                {
                    _currentScan = new ScanStatus
                    {
                        State = result.Success ? "completed" : "failed",
                        StartedAt = _currentScan?.StartedAt ?? DateTime.UtcNow,
                        CompletedAt = DateTime.UtcNow,
                        RootFolderId = id,
                        FilesFound = result.FilesFound,
                        FilesImported = result.FilesImported,
                        FilesRejected = result.FilesRejected,
                        Error = result.Error
                    };
                }
            }
            catch (Exception ex)
            {
                lock (_lock)
                {
                    _currentScan = new ScanStatus
                    {
                        State = "failed", StartedAt = _currentScan?.StartedAt ?? DateTime.UtcNow,
                        CompletedAt = DateTime.UtcNow, Error = ex.Message
                    };
                }
            }
        });

        return Accepted(new { message = "Movie scan started", pollUrl = "/api/v3/scan/movies/status" });
    }

    [HttpPost("library")]
    public ActionResult ScanLibrary()
    {
        lock (_lock)
        {
            if (_currentScan?.State == "scanning")
                return Conflict(new { error = "Scan already in progress", status = _currentScan });
            _currentScan = new ScanStatus { State = "scanning", StartedAt = DateTime.UtcNow };
        }

        _ = Task.Run(async () =>
        {
            try
            {
                var result = await _scanner.ScanLibraryAsync(_appLifetime.ApplicationStopping);
                lock (_lock)
                {
                    _currentScan = new ScanStatus
                    {
                        State = result.Success ? "completed" : "failed",
                        StartedAt = _currentScan?.StartedAt ?? DateTime.UtcNow,
                        CompletedAt = DateTime.UtcNow,
                        FilesFound = result.FilesFound,
                        FilesImported = result.FilesImported,
                        FilesRejected = result.FilesRejected,
                        Error = result.Error
                    };
                }
            }
            catch (Exception ex)
            {
                lock (_lock)
                {
                    _currentScan = new ScanStatus
                    {
                        State = "failed", StartedAt = _currentScan?.StartedAt ?? DateTime.UtcNow,
                        CompletedAt = DateTime.UtcNow, Error = ex.Message
                    };
                }
            }
        });

        return Accepted(new { message = "Movie library scan started", pollUrl = "/api/v3/scan/movies/status" });
    }
}

// ─── TV Scan ───────────────────────────────────────────────

[ApiController]
[Route("api/v3/scan/tv")]
[Authorize]
public class TVScanController : ControllerBase
{
    private readonly ITVShowScanner _scanner;
    private readonly IHostApplicationLifetime _appLifetime;
    private static ScanStatus? _currentScan;
    private static readonly object _lock = new();

    public TVScanController(ITVShowScanner scanner, IHostApplicationLifetime appLifetime)
    {
        _scanner = scanner;
        _appLifetime = appLifetime;
    }

    [HttpGet("status")]
    public ActionResult<ScanStatus> GetStatus()
    {
        lock (_lock) { return Ok(_currentScan ?? new ScanStatus { State = "idle" }); }
    }

    [HttpPost("rootfolder/{id:int}")]
    public ActionResult ScanRootFolder(int id)
    {
        lock (_lock)
        {
            if (_currentScan?.State == "scanning")
                return Conflict(new { error = "Scan already in progress", status = _currentScan });
            _currentScan = new ScanStatus { State = "scanning", StartedAt = DateTime.UtcNow, RootFolderId = id };
        }

        _ = Task.Run(async () =>
        {
            try
            {
                var result = await _scanner.ScanRootFolderAsync(id, _appLifetime.ApplicationStopping);
                lock (_lock)
                {
                    _currentScan = new ScanStatus
                    {
                        State = result.Success ? "completed" : "failed",
                        StartedAt = _currentScan?.StartedAt ?? DateTime.UtcNow,
                        CompletedAt = DateTime.UtcNow,
                        RootFolderId = id,
                        FilesFound = result.FilesFound,
                        FilesImported = result.FilesImported,
                        FilesRejected = result.FilesRejected,
                        Error = result.Error
                    };
                }
            }
            catch (Exception ex)
            {
                lock (_lock)
                {
                    _currentScan = new ScanStatus
                    {
                        State = "failed", StartedAt = _currentScan?.StartedAt ?? DateTime.UtcNow,
                        CompletedAt = DateTime.UtcNow, Error = ex.Message
                    };
                }
            }
        });

        return Accepted(new { message = "TV scan started", pollUrl = "/api/v3/scan/tv/status" });
    }

    [HttpPost("library")]
    public ActionResult ScanLibrary()
    {
        lock (_lock)
        {
            if (_currentScan?.State == "scanning")
                return Conflict(new { error = "Scan already in progress", status = _currentScan });
            _currentScan = new ScanStatus { State = "scanning", StartedAt = DateTime.UtcNow };
        }

        _ = Task.Run(async () =>
        {
            try
            {
                var result = await _scanner.ScanLibraryAsync(_appLifetime.ApplicationStopping);
                lock (_lock)
                {
                    _currentScan = new ScanStatus
                    {
                        State = result.Success ? "completed" : "failed",
                        StartedAt = _currentScan?.StartedAt ?? DateTime.UtcNow,
                        CompletedAt = DateTime.UtcNow,
                        FilesFound = result.FilesFound,
                        FilesImported = result.FilesImported,
                        FilesRejected = result.FilesRejected,
                        Error = result.Error
                    };
                }
            }
            catch (Exception ex)
            {
                lock (_lock)
                {
                    _currentScan = new ScanStatus
                    {
                        State = "failed", StartedAt = _currentScan?.StartedAt ?? DateTime.UtcNow,
                        CompletedAt = DateTime.UtcNow, Error = ex.Message
                    };
                }
            }
        });

        return Accepted(new { message = "TV library scan started", pollUrl = "/api/v3/scan/tv/status" });
    }
}

// ─── Book Scan ─────────────────────────────────────────────

[ApiController]
[Route("api/v3/scan/books")]
[Authorize]
public class BookScanController : ControllerBase
{
    private readonly IBookScanner _scanner;
    private readonly IHostApplicationLifetime _appLifetime;
    private static ScanStatus? _currentScan;
    private static readonly object _lock = new();

    public BookScanController(IBookScanner scanner, IHostApplicationLifetime appLifetime)
    {
        _scanner = scanner;
        _appLifetime = appLifetime;
    }

    [HttpGet("status")]
    public ActionResult<ScanStatus> GetStatus()
    {
        lock (_lock) { return Ok(_currentScan ?? new ScanStatus { State = "idle" }); }
    }

    [HttpPost("rootfolder/{id:int}")]
    public ActionResult ScanRootFolder(int id)
    {
        lock (_lock)
        {
            if (_currentScan?.State == "scanning")
                return Conflict(new { error = "Scan already in progress", status = _currentScan });
            _currentScan = new ScanStatus { State = "scanning", StartedAt = DateTime.UtcNow, RootFolderId = id };
        }

        _ = Task.Run(async () =>
        {
            try
            {
                var result = await _scanner.ScanRootFolderAsync(id, _appLifetime.ApplicationStopping);
                lock (_lock)
                {
                    _currentScan = new ScanStatus
                    {
                        State = result.Success ? "completed" : "failed",
                        StartedAt = _currentScan?.StartedAt ?? DateTime.UtcNow,
                        CompletedAt = DateTime.UtcNow,
                        RootFolderId = id,
                        FilesFound = result.FilesFound,
                        FilesImported = result.FilesImported,
                        FilesRejected = result.FilesRejected,
                        Error = result.Error
                    };
                }
            }
            catch (Exception ex)
            {
                lock (_lock)
                {
                    _currentScan = new ScanStatus
                    {
                        State = "failed", StartedAt = _currentScan?.StartedAt ?? DateTime.UtcNow,
                        CompletedAt = DateTime.UtcNow, Error = ex.Message
                    };
                }
            }
        });

        return Accepted(new { message = "Book scan started", pollUrl = "/api/v3/scan/books/status" });
    }

    [HttpPost("library")]
    public ActionResult ScanLibrary()
    {
        lock (_lock)
        {
            if (_currentScan?.State == "scanning")
                return Conflict(new { error = "Scan already in progress", status = _currentScan });
            _currentScan = new ScanStatus { State = "scanning", StartedAt = DateTime.UtcNow };
        }

        _ = Task.Run(async () =>
        {
            try
            {
                var result = await _scanner.ScanLibraryAsync(_appLifetime.ApplicationStopping);
                lock (_lock)
                {
                    _currentScan = new ScanStatus
                    {
                        State = result.Success ? "completed" : "failed",
                        StartedAt = _currentScan?.StartedAt ?? DateTime.UtcNow,
                        CompletedAt = DateTime.UtcNow,
                        FilesFound = result.FilesFound,
                        FilesImported = result.FilesImported,
                        FilesRejected = result.FilesRejected,
                        Error = result.Error
                    };
                }
            }
            catch (Exception ex)
            {
                lock (_lock)
                {
                    _currentScan = new ScanStatus
                    {
                        State = "failed", StartedAt = _currentScan?.StartedAt ?? DateTime.UtcNow,
                        CompletedAt = DateTime.UtcNow, Error = ex.Message
                    };
                }
            }
        });

        return Accepted(new { message = "Book library scan started", pollUrl = "/api/v3/scan/books/status" });
    }
}

// ─── Audiobook Scan ────────────────────────────────────────

[ApiController]
[Route("api/v3/scan/audiobooks")]
[Authorize]
public class AudiobookScanController : ControllerBase
{
    private readonly IAudiobookScanner _scanner;
    private readonly IHostApplicationLifetime _appLifetime;
    private static ScanStatus? _currentScan;
    private static readonly object _lock = new();

    public AudiobookScanController(IAudiobookScanner scanner, IHostApplicationLifetime appLifetime)
    {
        _scanner = scanner;
        _appLifetime = appLifetime;
    }

    [HttpGet("status")]
    public ActionResult<ScanStatus> GetStatus()
    {
        lock (_lock) { return Ok(_currentScan ?? new ScanStatus { State = "idle" }); }
    }

    [HttpPost("rootfolder/{id:int}")]
    public ActionResult ScanRootFolder(int id)
    {
        lock (_lock)
        {
            if (_currentScan?.State == "scanning")
                return Conflict(new { error = "Scan already in progress", status = _currentScan });
            _currentScan = new ScanStatus { State = "scanning", StartedAt = DateTime.UtcNow, RootFolderId = id };
        }

        _ = Task.Run(async () =>
        {
            try
            {
                var result = await _scanner.ScanRootFolderAsync(id, _appLifetime.ApplicationStopping);
                lock (_lock)
                {
                    _currentScan = new ScanStatus
                    {
                        State = result.Success ? "completed" : "failed",
                        StartedAt = _currentScan?.StartedAt ?? DateTime.UtcNow,
                        CompletedAt = DateTime.UtcNow,
                        RootFolderId = id,
                        FilesFound = result.FilesFound,
                        FilesImported = result.FilesImported,
                        FilesRejected = result.FilesRejected,
                        Error = result.Error
                    };
                }
            }
            catch (Exception ex)
            {
                lock (_lock)
                {
                    _currentScan = new ScanStatus
                    {
                        State = "failed", StartedAt = _currentScan?.StartedAt ?? DateTime.UtcNow,
                        CompletedAt = DateTime.UtcNow, Error = ex.Message
                    };
                }
            }
        });

        return Accepted(new { message = "Audiobook scan started", pollUrl = "/api/v3/scan/audiobooks/status" });
    }

    [HttpPost("library")]
    public ActionResult ScanLibrary()
    {
        lock (_lock)
        {
            if (_currentScan?.State == "scanning")
                return Conflict(new { error = "Scan already in progress", status = _currentScan });
            _currentScan = new ScanStatus { State = "scanning", StartedAt = DateTime.UtcNow };
        }

        _ = Task.Run(async () =>
        {
            try
            {
                var result = await _scanner.ScanLibraryAsync(_appLifetime.ApplicationStopping);
                lock (_lock)
                {
                    _currentScan = new ScanStatus
                    {
                        State = result.Success ? "completed" : "failed",
                        StartedAt = _currentScan?.StartedAt ?? DateTime.UtcNow,
                        CompletedAt = DateTime.UtcNow,
                        FilesFound = result.FilesFound,
                        FilesImported = result.FilesImported,
                        FilesRejected = result.FilesRejected,
                        Error = result.Error
                    };
                }
            }
            catch (Exception ex)
            {
                lock (_lock)
                {
                    _currentScan = new ScanStatus
                    {
                        State = "failed", StartedAt = _currentScan?.StartedAt ?? DateTime.UtcNow,
                        CompletedAt = DateTime.UtcNow, Error = ex.Message
                    };
                }
            }
        });

        return Accepted(new { message = "Audiobook library scan started", pollUrl = "/api/v3/scan/audiobooks/status" });
    }
}

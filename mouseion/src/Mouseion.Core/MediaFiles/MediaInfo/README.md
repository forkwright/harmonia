# MediaInfo Module

FFprobe wrapper for extracting video/audio metadata from media files.

## Components

### Models

**HdrFormat.cs**
- Enum for HDR format detection (None, HDR10, HDR10+, Dolby Vision variants, HLG, PQ10)

**MediaInfoModel.cs**
- Complete metadata model including:
  - Container format
  - Video: codec, profile, bitrate, bit depth, resolution, HDR format, frame rate, color primaries
  - Audio: codec, profile, bitrate, channels, channel layout, languages
  - Subtitles: language list
  - Runtime calculation
  - Raw FFprobe JSON output (for debugging)

### Services

**IMediaInfoService / MediaInfoService**
- FFprobe wrapper with JSON parsing
- Auto-detects ffprobe location (bundled, system path)
- Handles missing FFprobe gracefully
- Dual-pass analysis for complex files (audio channel layout detection)
- Frame-level analysis for HDR metadata
- Schema versioning (revision 14)

**IUpdateMediaInfoService / UpdateMediaInfoService**
- Updates MediaFile entities with MediaInfo
- Path resolution (absolute or relative)
- Persistence via MediaFileRepository
- Integration point for file import pipeline

**MediaInfoFormatter**
- Human-readable codec names (h264 → "x264", dts → "DTS-HD MA")
- Scene name matching for codec detection
- HDR format display strings
- Audio channel parsing from position strings

## Usage

### Extract MediaInfo

```csharp
var mediaInfoService = container.Resolve<IMediaInfoService>();
var info = mediaInfoService.GetMediaInfo("/path/to/movie.mkv");

Console.WriteLine($"Video: {info.VideoFormat} {info.Width}x{info.Height}");
Console.WriteLine($"Audio: {info.AudioFormat} {info.AudioChannels}ch");
Console.WriteLine($"Runtime: {info.RunTime}");
Console.WriteLine($"HDR: {info.VideoHdrFormat}");
```

### Update MediaFile

```csharp
var updateService = container.Resolve<IUpdateMediaInfoService>();
var mediaFile = new MediaFile { Path = "/path/to/movie.mkv" };
updateService.Update(mediaFile, "/media/movies");
```

### Format for Display

```csharp
var videoCodec = MediaInfoFormatter.FormatVideoCodec(mediaInfo, sceneName, logger);
var audioCodec = MediaInfoFormatter.FormatAudioCodec(mediaInfo, sceneName, logger);
var hdrType = MediaInfoFormatter.FormatVideoDynamicRangeType(mediaInfo);

Console.WriteLine($"{videoCodec} / {audioCodec} / {hdrType}");
// Output: "x265 / DTS-HD MA / DV HDR10"
```

## FFprobe Detection

Service checks these paths in order:
1. `ffprobe` (system PATH)
2. `/usr/bin/ffprobe`
3. `/usr/local/bin/ffprobe`
4. `{AppDomain.BaseDirectory}/ffprobe`
5. `{AppDomain.BaseDirectory}/ffprobe.exe`

If none found, throws `InvalidOperationException` on first use.

## HDR Detection Logic

1. Bit depth must be ≥10
2. Dolby Vision detected via side data (DOVI)
3. HDR10+ detected via dynamic metadata (SMPTE2094)
4. HDR10 detected via mastering metadata
5. HLG detected via transfer function (arib-std-b67)
6. PQ10 detected via transfer function (smpte2084)
7. Requires bt2020 color primaries

## Schema Versioning

- Current: 14
- Minimum: 14
- Used to invalidate cached MediaInfo when format changes
- Stored in `MediaInfoModel.SchemaRevision`

## Error Handling

- Missing files: `FileNotFoundException`
- Missing ffprobe: `InvalidOperationException`
- Parse failures: Returns `null`, logs error
- Disk images (.iso, .img): Returns `null` (unsupported)

## Integration Points

- **File Import**: `ImportApprovedFiles` → `UpdateMediaInfoService`
- **Library Scan**: `DiskScanService` → triggers MediaInfo update
- **Quality Detection**: Uses `MediaInfoModel` for resolution/codec matching
- **API**: Exposes `MediaInfoModel` in MediaFile endpoints

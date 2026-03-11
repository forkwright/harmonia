# News (RSS/Atom Feed Aggregation)

> Related: [data/media-schemas.md](../data/media-schemas.md) for table definitions, [data/want-release.md](../data/want-release.md) for the News Exception, [media/lifecycle.md](lifecycle.md) for lifecycle state machine.

## Overview

News feeds are RSS/Atom sources aggregated into the Harmonia library. Articles are fetched on schedule, stored locally, and presented alongside other media types in the unified library view.

News is the eighth media type. It sits alongside podcasts in the feed-based category — both use RSS/Atom as a transport — but they are distinct types. Podcasts have audio enclosures; news has text articles. They share no tables.

## Acquisition

- User adds feed URLs manually or via OPML import
- Syntaxis schedules periodic feed fetches (default: hourly, configurable per feed)
- feed-rs parses both RSS 2.0 and Atom 1.0
- Articles stored with full content when the feed provides it; summary otherwise
- Full article body extraction via readability is deferred to Phase 5

Feed fetching does not use the want/release/have pipeline. There is no search or grab step — the feed URL is the source. See `want-release.md` News Exception.

## Metadata

**Feed-level** metadata comes directly from the feed document:

| Field | Source |
|-------|--------|
| `title` | `<title>` element |
| `site_url` | `<link>` element (the website, not the feed URL) |
| `description` | `<description>` / `<subtitle>` element |
| `icon_url` | `<image>` / `<icon>` element |

**Article-level** metadata:

| Field | Source |
|-------|--------|
| `title` | `<title>` element |
| `url` | `<link>` element |
| `author` | `<author>` / `<dc:creator>` element |
| `content_html` | `<content:encoded>` or `<content>` element |
| `summary` | `<description>` element (RSS) / `<summary>` (Atom) |
| `published_at` | `<pubDate>` (RSS) / `<published>` (Atom) |
| `guid` | `<guid>` (RSS) / `<id>` (Atom) — deduplication key |

No external metadata provider is required. The feed is self-describing.

## Integration

- Podcast feeds and news feeds both use feed-rs but are separate media types with separate tables
- Podcasts have enclosures (audio files); news has articles (text/HTML)
- OPDS does not serve news — no OPDS standard exists for article feeds
- `source_type` on `news_articles` is `rss` or `atom` — set by feed-rs at parse time

## Read State

`is_read` and `is_starred` are per-article boolean columns. In v1 (single-user), these are stored flat on `news_articles`. Multi-user read state is deferred — the column structure matches `podcast_episodes.listened` for consistency.

## OPML Import

OPML is the standard interchange format for feed lists. Import parses `<outline>` elements with `type="rss"` and creates `news_feeds` rows. Duplicate feed URLs (matching `news_feeds.url`) are skipped without error.

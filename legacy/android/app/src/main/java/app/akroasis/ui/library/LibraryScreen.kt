package app.akroasis.ui.library

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import app.akroasis.data.model.Album
import app.akroasis.data.model.Artist
import app.akroasis.data.model.MediaType
import app.akroasis.data.model.Track
import app.akroasis.ui.player.PlayerViewModel

@Composable
fun LibraryScreen(
    libraryViewModel: LibraryViewModel = hiltViewModel(),
    playerViewModel: PlayerViewModel = hiltViewModel(),
    modifier: Modifier = Modifier
) {
    var selectedMediaType by remember { mutableStateOf(MediaType.MUSIC) }

    Column(modifier = modifier.fillMaxSize()) {
        TabRow(selectedTabIndex = selectedMediaType.ordinal) {
            Tab(
                selected = selectedMediaType == MediaType.MUSIC,
                onClick = { selectedMediaType = MediaType.MUSIC },
                text = { Text("Music") }
            )
            Tab(
                selected = selectedMediaType == MediaType.AUDIOBOOK,
                onClick = { selectedMediaType = MediaType.AUDIOBOOK },
                text = { Text("Audiobooks") }
            )
            Tab(
                selected = selectedMediaType == MediaType.EBOOK,
                onClick = { selectedMediaType = MediaType.EBOOK },
                text = { Text("Ebooks") }
            )
        }

        when (selectedMediaType) {
            MediaType.MUSIC -> MusicLibraryContent(
                libraryViewModel = libraryViewModel,
                playerViewModel = playerViewModel
            )
            MediaType.AUDIOBOOK -> PlaceholderContent("Audiobook library coming soon")
            MediaType.EBOOK -> PlaceholderContent("Ebook library coming soon")
        }
    }
}

@Composable
private fun MusicLibraryContent(
    libraryViewModel: LibraryViewModel,
    playerViewModel: PlayerViewModel,
    modifier: Modifier = Modifier
) {
    var selectedArtist by remember { mutableStateOf<Artist?>(null) }
    var selectedAlbum by remember { mutableStateOf<Album?>(null) }

    Surface(
        modifier = modifier.fillMaxSize(),
        color = MaterialTheme.colorScheme.background
    ) {
        when {
            selectedAlbum != null -> {
                selectedAlbum?.let { album ->
                    TrackListScreen(
                        albumId = album.id,
                        albumTitle = album.title,
                        onBack = { selectedAlbum = null },
                        onTrackClick = { track, allTracks ->
                            val trackIndex = allTracks.indexOf(track)
                            playerViewModel.playTracks(allTracks, trackIndex)
                        },
                        libraryViewModel = libraryViewModel
                    )
                }
            }
            selectedArtist != null -> {
                selectedArtist?.let { artist ->
                    AlbumListScreen(
                        artistId = artist.id,
                        artistName = artist.name,
                        onBack = { selectedArtist = null },
                        onAlbumClick = { album ->
                            selectedAlbum = album
                        },
                        libraryViewModel = libraryViewModel
                    )
                }
            }
            else -> {
                ArtistListScreen(
                    onArtistClick = { artist ->
                        selectedArtist = artist
                        libraryViewModel.loadAlbums(artist.id)
                    },
                    libraryViewModel = libraryViewModel
                )
            }
        }
    }
}

@Composable
private fun PlaceholderContent(
    message: String,
    modifier: Modifier = Modifier
) {
    Box(
        modifier = modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Text(
            text = message,
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(32.dp)
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ArtistListScreen(
    onArtistClick: (Artist) -> Unit,
    libraryViewModel: LibraryViewModel
) {
    val artistsState by libraryViewModel.artistsState.collectAsState()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Artists") }
            )
        }
    ) { padding ->
        when (val state = artistsState) {
            is LibraryState.Loading -> {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding),
                    contentAlignment = Alignment.Center
                ) {
                    CircularProgressIndicator()
                }
            }
            is LibraryState.Success -> {
                LazyColumn(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding)
                ) {
                    items(state.data) { artist ->
                        ArtistListItem(
                            artist = artist,
                            onClick = { onArtistClick(artist) }
                        )
                    }
                }
            }
            is LibraryState.Error -> {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding),
                    contentAlignment = Alignment.Center
                ) {
                    Text(
                        text = state.message,
                        color = MaterialTheme.colorScheme.error
                    )
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AlbumListScreen(
    artistId: String,
    artistName: String,
    onBack: () -> Unit,
    onAlbumClick: (Album) -> Unit,
    libraryViewModel: LibraryViewModel
) {
    val albumsState by libraryViewModel.albumsState.collectAsState()

    LaunchedEffect(artistId) {
        libraryViewModel.loadAlbums(artistId = artistId)
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(artistName) },
                navigationIcon = {
                    TextButton(onClick = onBack) {
                        Text("Back")
                    }
                }
            )
        }
    ) { padding ->
        when (val state = albumsState) {
            is LibraryState.Loading -> {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding),
                    contentAlignment = Alignment.Center
                ) {
                    CircularProgressIndicator()
                }
            }
            is LibraryState.Success -> {
                LazyColumn(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding)
                ) {
                    items(state.data) { album ->
                        AlbumListItem(
                            album = album,
                            onClick = { onAlbumClick(album) }
                        )
                    }
                }
            }
            is LibraryState.Error -> {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding),
                    contentAlignment = Alignment.Center
                ) {
                    Text(
                        text = state.message,
                        color = MaterialTheme.colorScheme.error
                    )
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TrackListScreen(
    albumId: String,
    albumTitle: String,
    onBack: () -> Unit,
    onTrackClick: (Track, List<Track>) -> Unit,
    libraryViewModel: LibraryViewModel
) {
    val tracksState by libraryViewModel.tracksState.collectAsState()

    LaunchedEffect(albumId) {
        libraryViewModel.loadTracks(albumId = albumId)
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(albumTitle) },
                navigationIcon = {
                    TextButton(onClick = onBack) {
                        Text("Back")
                    }
                }
            )
        }
    ) { padding ->
        when (val state = tracksState) {
            is LibraryState.Loading -> {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding),
                    contentAlignment = Alignment.Center
                ) {
                    CircularProgressIndicator()
                }
            }
            is LibraryState.Success -> {
                val tracks = state.data
                LazyColumn(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding)
                ) {
                    items(tracks) { track ->
                        TrackListItem(
                            track = track,
                            onClick = { onTrackClick(track, tracks) }
                        )
                    }
                }
            }
            is LibraryState.Error -> {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding),
                    contentAlignment = Alignment.Center
                ) {
                    Text(
                        text = state.message,
                        color = MaterialTheme.colorScheme.error
                    )
                }
            }
        }
    }
}

@Composable
fun ArtistListItem(
    artist: Artist,
    onClick: () -> Unit
) {
    ListItem(
        headlineContent = {
            Text(
                text = artist.name,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
        },
        supportingContent = {
            Text("${artist.albumCount} albums • ${artist.trackCount} tracks")
        },
        modifier = Modifier.clickable(onClick = onClick)
    )
}

@Composable
fun AlbumListItem(
    album: Album,
    onClick: () -> Unit
) {
    ListItem(
        headlineContent = {
            Text(
                text = album.title,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
        },
        supportingContent = {
            Text("${album.year ?: "Unknown"} • ${album.trackCount} tracks")
        },
        modifier = Modifier.clickable(onClick = onClick)
    )
}

@Composable
fun TrackListItem(
    track: Track,
    onClick: () -> Unit
) {
    ListItem(
        headlineContent = {
            Text(
                text = track.title,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
        },
        supportingContent = {
            val duration = formatDuration(track.duration)
            val quality = track.sampleRate?.let { sr ->
                val khz = sr / 1000
                "${khz}kHz"
            } ?: ""
            Text("$duration${if (quality.isNotEmpty()) " • $quality" else ""}")
        },
        leadingContent = {
            Text(
                text = "${track.trackNumber ?: 0}",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        },
        modifier = Modifier.clickable(onClick = onClick)
    )
}

private fun formatDuration(ms: Long): String {
    val totalSeconds = ms / 1000
    val minutes = totalSeconds / 60
    val seconds = totalSeconds % 60
    return "%d:%02d".format(minutes, seconds)
}

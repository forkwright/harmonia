// Readium-based EPUB reader screen
package app.akroasis.ui.ebook

import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.fragment.app.FragmentActivity
import androidx.fragment.app.FragmentContainerView
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import org.readium.r2.navigator.epub.EpubDefaults
import org.readium.r2.navigator.epub.EpubNavigatorFactory
import org.readium.r2.navigator.epub.EpubNavigatorFragment
import org.readium.r2.navigator.epub.EpubPreferences
import org.readium.r2.shared.util.AbsoluteUrl
import timber.log.Timber

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EbookReaderScreen(
    ebookId: String,
    onNavigateBack: () -> Unit,
    viewModel: EbookViewModel = hiltViewModel()
) {
    val context = LocalContext.current
    val activity = context as? FragmentActivity
        ?: run {
            Text("Error: Not running in FragmentActivity")
            return
        }

    val currentEbook by viewModel.currentEbook.collectAsStateWithLifecycle()
    val publication by viewModel.publication.collectAsStateWithLifecycle()
    val progress by viewModel.progress.collectAsStateWithLifecycle()
    val isLoading by viewModel.isLoading.collectAsStateWithLifecycle()
    val error by viewModel.error.collectAsStateWithLifecycle()

    LaunchedEffect(ebookId) {
        viewModel.loadEbook(ebookId)
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text(
                            text = currentEbook?.title ?: "Loading...",
                            style = MaterialTheme.typography.titleMedium
                        )
                        progress?.let { prog ->
                            Text(
                                text = "${(prog.percentComplete * 100).toInt()}% complete",
                                style = MaterialTheme.typography.bodySmall
                            )
                        }
                    }
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back")
                    }
                }
            )
        }
    ) { paddingValues ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            when {
                error != null -> {
                    ErrorView(
                        message = error ?: "Unknown error",
                        onRetry = { viewModel.loadEbook(ebookId) },
                        onDismiss = { viewModel.clearError() }
                    )
                }
                isLoading || publication == null -> {
                    LoadingView()
                }
                else -> {
                    EpubReaderContent(
                        publication = publication!!,
                        fragmentManager = activity.supportFragmentManager,
                        savedLocator = viewModel.savedLocator.collectAsState().value,
                        onNavigatorReady = { navigator ->
                            viewModel.setNavigator(navigator)
                        }
                    )
                }
            }
        }
    }
}

@Composable
private fun EpubReaderContent(
    publication: org.readium.r2.shared.publication.Publication,
    fragmentManager: androidx.fragment.app.FragmentManager,
    savedLocator: org.readium.r2.shared.publication.Locator?,
    onNavigatorReady: (org.readium.r2.navigator.Navigator) -> Unit
) {
    AndroidView(
        factory = { context ->
            FragmentContainerView(context).apply {
                id = android.view.View.generateViewId()

                // Create navigator factory with Readium 3.x API
                val navigatorFactory = EpubNavigatorFactory(
                    publication = publication,
                    configuration = EpubNavigatorFactory.Configuration(
                        defaults = EpubDefaults(
                            pageMargins = 1.5,
                            scroll = false
                        )
                    )
                )

                // Create fragment factory with listener
                val fragmentFactory = navigatorFactory.createFragmentFactory(
                    initialLocator = savedLocator,
                    initialPreferences = EpubPreferences(),
                    listener = object : EpubNavigatorFragment.Listener {
                        override fun onExternalLinkActivated(url: AbsoluteUrl) {
                            Timber.d("External link activated: $url")
                        }
                    }
                )

                // Set custom fragment factory before creating fragment
                fragmentManager.fragmentFactory = fragmentFactory

                // Create and add the fragment
                fragmentManager.beginTransaction()
                    .replace(this.id, EpubNavigatorFragment::class.java, null)
                    .commitNow()

                // Get the created fragment - the fragment IS the navigator
                val fragment = fragmentManager.findFragmentById(this.id) as? EpubNavigatorFragment
                fragment?.let { epubFragment ->
                    // The fragment itself implements Navigator interfaces
                    onNavigatorReady(epubFragment)
                    Timber.d("Readium Navigator initialized")
                } ?: Timber.w("Navigator fragment not found after creation")
            }
        },
        modifier = Modifier.fillMaxSize()
    )
}

@Composable
private fun LoadingView() {
    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            CircularProgressIndicator()
            Text("Loading EPUB...", style = MaterialTheme.typography.bodyLarge)
        }
    }
}

@Composable
private fun ErrorView(
    message: String,
    onRetry: () -> Unit,
    onDismiss: () -> Unit
) {
    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Card(
            modifier = Modifier
                .padding(16.dp)
                .fillMaxWidth()
        ) {
            Column(
                modifier = Modifier.padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                Text(
                    text = "Error Loading EPUB",
                    style = MaterialTheme.typography.titleLarge,
                    color = MaterialTheme.colorScheme.error
                )
                Text(
                    text = message,
                    style = MaterialTheme.typography.bodyMedium
                )
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    OutlinedButton(
                        onClick = onDismiss,
                        modifier = Modifier.weight(1f)
                    ) {
                        Text("Dismiss")
                    }
                    Button(
                        onClick = onRetry,
                        modifier = Modifier.weight(1f)
                    ) {
                        Text("Retry")
                    }
                }
            }
        }
    }
}

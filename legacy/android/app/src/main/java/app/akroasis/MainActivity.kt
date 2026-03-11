package app.akroasis

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.PlayCircle
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.hilt.navigation.compose.hiltViewModel
import app.akroasis.permissions.PermissionManager
import app.akroasis.ui.auth.AuthState
import app.akroasis.ui.auth.AuthViewModel
import app.akroasis.ui.auth.LoginScreen
import app.akroasis.ui.continuefeed.ContinueFeedScreen
import app.akroasis.ui.library.LibraryScreen
import app.akroasis.ui.player.NowPlayingScreen
import app.akroasis.ui.theme.AkroasisTheme
import dagger.hilt.android.AndroidEntryPoint
import javax.inject.Inject

@AndroidEntryPoint
class MainActivity : ComponentActivity() {

    @Inject
    lateinit var permissionManager: PermissionManager

    private val permissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions()
    ) { permissions ->
        val deniedPermissions = permissions.filterValues { !it }.keys
        if (deniedPermissions.isNotEmpty()) {
            // Could show a dialog explaining why permissions are needed
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        // Request permissions if needed
        if (!permissionManager.hasRequiredPermissions()) {
            val missingPermissions = permissionManager.getMissingPermissions()
            permissionLauncher.launch(missingPermissions)
        }

        setContent {
            AkroasisTheme {
                AppRoot()
            }
        }
    }
}

@Composable
fun AppRoot(
    authViewModel: AuthViewModel = hiltViewModel()
) {
    val authState by authViewModel.authState.collectAsState()

    when (authState) {
        is AuthState.Authenticated -> MainScreen()
        else -> LoginScreen(onLoginSuccess = {})
    }
}

@Composable
fun MainScreen() {
    var selectedTab by remember { mutableIntStateOf(0) }

    Scaffold(
        modifier = Modifier.fillMaxSize(),
        bottomBar = {
            NavigationBar {
                NavigationBarItem(
                    icon = { Icon(Icons.Default.PlayCircle, contentDescription = "Continue") },
                    label = { Text("Continue") },
                    selected = selectedTab == 0,
                    onClick = { selectedTab = 0 }
                )
                NavigationBarItem(
                    icon = { Icon(Icons.Default.Home, contentDescription = "Library") },
                    label = { Text("Library") },
                    selected = selectedTab == 1,
                    onClick = { selectedTab = 1 }
                )
                NavigationBarItem(
                    icon = { Icon(Icons.Default.PlayArrow, contentDescription = "Now Playing") },
                    label = { Text("Now Playing") },
                    selected = selectedTab == 2,
                    onClick = { selectedTab = 2 }
                )
            }
        }
    ) { innerPadding ->
        when (selectedTab) {
            0 -> ContinueFeedScreen(
                onNavigateToPlayer = { selectedTab = 2 },  // Switch to Now Playing tab
                onNavigateToAudiobook = { selectedTab = 2 },  // Switch to Now Playing tab
                onNavigateToEbook = { },
                modifier = Modifier.padding(innerPadding)
            )
            1 -> LibraryScreen(
                modifier = Modifier.padding(innerPadding)
            )
            2 -> NowPlayingScreen(
                modifier = Modifier.padding(innerPadding)
            )
        }
    }
}

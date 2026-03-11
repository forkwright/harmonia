// Login screen for Mouseion server authentication
package app.akroasis.ui.auth

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Visibility
import androidx.compose.material.icons.filled.VisibilityOff
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusDirection
import androidx.compose.ui.focus.FocusManager
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel

@Composable
fun LoginScreen(
    viewModel: AuthViewModel = hiltViewModel(),
    onLoginSuccess: () -> Unit
) {
    val authState by viewModel.authState.collectAsState()
    val focusManager = LocalFocusManager.current

    var serverUrl by remember { mutableStateOf("") }
    var username by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var passwordVisible by remember { mutableStateOf(false) }

    LaunchedEffect(authState) {
        if (authState is AuthState.Authenticated) {
            onLoginSuccess()
        }
    }

    Surface(
        modifier = Modifier.fillMaxSize(),
        color = MaterialTheme.colorScheme.background
    ) {
        LoginContent(
            authState = authState,
            serverUrl = serverUrl,
            username = username,
            password = password,
            passwordVisible = passwordVisible,
            focusManager = focusManager,
            onServerUrlChange = { serverUrl = it },
            onUsernameChange = { username = it },
            onPasswordChange = { password = it },
            onPasswordVisibilityToggle = { passwordVisible = !passwordVisible },
            onLogin = { viewModel.login(username, password, serverUrl) }
        )
    }
}

@Composable
private fun LoginContent(
    authState: AuthState,
    serverUrl: String,
    username: String,
    password: String,
    passwordVisible: Boolean,
    focusManager: FocusManager,
    onServerUrlChange: (String) -> Unit,
    onUsernameChange: (String) -> Unit,
    onPasswordChange: (String) -> Unit,
    onPasswordVisibilityToggle: () -> Unit,
    onLogin: () -> Unit
) {
    val isLoading = authState is AuthState.Loading

    Box(
        modifier = Modifier
            .fillMaxSize()
            .padding(32.dp),
        contentAlignment = Alignment.Center
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(16.dp),
            modifier = Modifier.fillMaxWidth()
        ) {
            LoginHeader()
            Spacer(modifier = Modifier.height(16.dp))
            ServerUrlField(
                value = serverUrl,
                onValueChange = onServerUrlChange,
                enabled = !isLoading,
                focusManager = focusManager
            )
            UsernameField(
                value = username,
                onValueChange = onUsernameChange,
                enabled = !isLoading,
                focusManager = focusManager
            )
            PasswordField(
                value = password,
                onValueChange = onPasswordChange,
                passwordVisible = passwordVisible,
                onVisibilityToggle = onPasswordVisibilityToggle,
                enabled = !isLoading,
                focusManager = focusManager,
                onDone = onLogin
            )
            ErrorMessage(authState = authState)
            Spacer(modifier = Modifier.height(8.dp))
            LoginButton(isLoading = isLoading, focusManager = focusManager, onLogin = onLogin)
        }
    }
}

@Composable
private fun LoginHeader() {
    Text(
        text = "Akroasis",
        style = MaterialTheme.typography.headlineLarge,
        color = MaterialTheme.colorScheme.primary
    )
    Text(
        text = "Connect to Mouseion",
        style = MaterialTheme.typography.bodyMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant
    )
}

@Composable
private fun ServerUrlField(
    value: String,
    onValueChange: (String) -> Unit,
    enabled: Boolean,
    focusManager: FocusManager
) {
    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        label = { Text("Server URL") },
        placeholder = { Text("https://example.com:5000") },
        singleLine = true,
        keyboardOptions = KeyboardOptions(
            keyboardType = KeyboardType.Uri,
            imeAction = ImeAction.Next
        ),
        keyboardActions = KeyboardActions(
            onNext = { focusManager.moveFocus(FocusDirection.Down) }
        ),
        modifier = Modifier.fillMaxWidth(),
        enabled = enabled
    )
}

@Composable
private fun UsernameField(
    value: String,
    onValueChange: (String) -> Unit,
    enabled: Boolean,
    focusManager: FocusManager
) {
    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        label = { Text("Username") },
        singleLine = true,
        keyboardOptions = KeyboardOptions(
            keyboardType = KeyboardType.Text,
            imeAction = ImeAction.Next
        ),
        keyboardActions = KeyboardActions(
            onNext = { focusManager.moveFocus(FocusDirection.Down) }
        ),
        modifier = Modifier.fillMaxWidth(),
        enabled = enabled
    )
}

@Composable
private fun PasswordField(
    value: String,
    onValueChange: (String) -> Unit,
    passwordVisible: Boolean,
    onVisibilityToggle: () -> Unit,
    enabled: Boolean,
    focusManager: FocusManager,
    onDone: () -> Unit
) {
    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        label = { Text("Password") },
        singleLine = true,
        visualTransformation = if (passwordVisible) {
            VisualTransformation.None
        } else {
            PasswordVisualTransformation()
        },
        keyboardOptions = KeyboardOptions(
            keyboardType = KeyboardType.Password,
            imeAction = ImeAction.Done
        ),
        keyboardActions = KeyboardActions(
            onDone = {
                focusManager.clearFocus()
                onDone()
            }
        ),
        trailingIcon = {
            PasswordVisibilityToggle(
                passwordVisible = passwordVisible,
                onToggle = onVisibilityToggle
            )
        },
        modifier = Modifier.fillMaxWidth(),
        enabled = enabled
    )
}

@Composable
private fun PasswordVisibilityToggle(
    passwordVisible: Boolean,
    onToggle: () -> Unit
) {
    IconButton(onClick = onToggle) {
        Icon(
            imageVector = if (passwordVisible) Icons.Default.Visibility else Icons.Default.VisibilityOff,
            contentDescription = if (passwordVisible) "Hide password" else "Show password"
        )
    }
}

@Composable
private fun ErrorMessage(authState: AuthState) {
    if (authState is AuthState.Error) {
        Text(
            text = authState.message,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.error,
            modifier = Modifier.padding(vertical = 8.dp)
        )
    }
}

@Composable
private fun LoginButton(
    isLoading: Boolean,
    focusManager: FocusManager,
    onLogin: () -> Unit
) {
    Button(
        onClick = {
            focusManager.clearFocus()
            onLogin()
        },
        enabled = !isLoading,
        modifier = Modifier
            .fillMaxWidth()
            .height(56.dp)
    ) {
        if (isLoading) {
            CircularProgressIndicator(
                modifier = Modifier.size(24.dp),
                color = MaterialTheme.colorScheme.onPrimary
            )
        } else {
            Text("Login")
        }
    }
}

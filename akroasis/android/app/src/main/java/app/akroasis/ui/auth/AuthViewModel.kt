package app.akroasis.ui.auth

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.akroasis.data.api.AuthInterceptor
import app.akroasis.data.api.LoginRequest
import app.akroasis.data.api.MouseionApi
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import javax.inject.Inject

@HiltViewModel
class AuthViewModel @Inject constructor(
    private val api: MouseionApi,
    private val authInterceptor: AuthInterceptor
) : ViewModel() {

    private val _authState = MutableStateFlow<AuthState>(AuthState.Unauthenticated)
    val authState: StateFlow<AuthState> = _authState.asStateFlow()

    init {
        checkAuthentication()
    }

    private fun checkAuthentication() {
        val refreshToken = authInterceptor.getRefreshToken()
        if (refreshToken != null) {
            _authState.value = AuthState.Authenticated
        } else {
            _authState.value = AuthState.Unauthenticated
        }
    }

    fun login(username: String, password: String, serverUrl: String) {
        if (username.isBlank() || password.isBlank() || serverUrl.isBlank()) {
            _authState.value = AuthState.Error("Username, password, and server URL are required")
            return
        }

        viewModelScope.launch {
            _authState.value = AuthState.Loading

            try {
                val response = api.login(LoginRequest(username, password))

                if (response.isSuccessful && response.body() != null) {
                    val loginResponse = response.body()!!
                    authInterceptor.saveTokens(
                        loginResponse.accessToken,
                        loginResponse.refreshToken
                    )
                    authInterceptor.saveServerUrl(serverUrl)
                    _authState.value = AuthState.Authenticated
                } else {
                    val errorMsg = when (response.code()) {
                        401 -> "Invalid username or password"
                        404 -> "Server not found. Check the URL."
                        else -> "Login failed: ${response.code()}"
                    }
                    _authState.value = AuthState.Error(errorMsg)
                }
            } catch (e: Exception) {
                _authState.value = AuthState.Error(
                    "Connection failed: ${e.message ?: "Unknown error"}"
                )
            }
        }
    }

    fun logout() {
        authInterceptor.clearTokens()
        _authState.value = AuthState.Unauthenticated
    }
}

sealed class AuthState {
    data object Unauthenticated : AuthState()
    data object Loading : AuthState()
    data object Authenticated : AuthState()
    data class Error(val message: String) : AuthState()
}

// Network connectivity and quality monitoring
package app.akroasis.network

import android.Manifest
import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import androidx.annotation.RequiresPermission
import dagger.hilt.android.qualifiers.ApplicationContext
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.callbackFlow
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class NetworkMonitor @Inject constructor(
    @ApplicationContext private val context: Context
) {
    private val connectivityManager = context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager

    sealed class NetworkType {
        object WiFi : NetworkType()
        object Cellular : NetworkType()
        object Ethernet : NetworkType()
        object None : NetworkType()
        object Unknown : NetworkType()
    }

    data class NetworkState(
        val type: NetworkType,
        val isConnected: Boolean,
        val isMetered: Boolean,
        val linkDownstreamBandwidthKbps: Int = 0
    )

    @RequiresPermission(Manifest.permission.ACCESS_NETWORK_STATE)
    fun observeNetworkState(): Flow<NetworkState> = callbackFlow {
        val callback = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) {
                trySend(getCurrentNetworkState())
            }

            override fun onLost(network: Network) {
                trySend(NetworkState(
                    type = NetworkType.None,
                    isConnected = false,
                    isMetered = false
                ))
            }

            override fun onCapabilitiesChanged(
                network: Network,
                capabilities: NetworkCapabilities
            ) {
                trySend(getCurrentNetworkState())
            }
        }

        val request = NetworkRequest.Builder()
            .addCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
            .build()

        connectivityManager.registerNetworkCallback(request, callback)

        trySend(getCurrentNetworkState())

        awaitClose {
            connectivityManager.unregisterNetworkCallback(callback)
        }
    }

    fun getCurrentNetworkState(): NetworkState {
        val activeNetwork = connectivityManager.activeNetwork
        val capabilities = connectivityManager.getNetworkCapabilities(activeNetwork)

        if (capabilities == null || activeNetwork == null) {
            return NetworkState(
                type = NetworkType.None,
                isConnected = false,
                isMetered = false
            )
        }

        val networkType = when {
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) -> NetworkType.WiFi
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR) -> NetworkType.Cellular
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET) -> NetworkType.Ethernet
            else -> NetworkType.Unknown
        }

        val isMetered = !capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED)
        val bandwidth = capabilities.linkDownstreamBandwidthKbps

        return NetworkState(
            type = networkType,
            isConnected = true,
            isMetered = isMetered,
            linkDownstreamBandwidthKbps = bandwidth
        )
    }

    fun isConnected(): Boolean {
        return getCurrentNetworkState().isConnected
    }

    fun isWiFi(): Boolean {
        return getCurrentNetworkState().type is NetworkType.WiFi
    }

    fun isCellular(): Boolean {
        return getCurrentNetworkState().type is NetworkType.Cellular
    }

    fun isMetered(): Boolean {
        return getCurrentNetworkState().isMetered
    }
}

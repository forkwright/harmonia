// Dagger Hilt dependency injection module
package app.akroasis.di

import android.content.Context
import androidx.room.Room
import app.akroasis.audio.AudioPlayer
import app.akroasis.data.api.AuthInterceptor
import app.akroasis.data.api.MouseionApi
import app.akroasis.data.local.MusicDatabase
import app.akroasis.data.preferences.ServerPreferences
import com.google.gson.Gson
import com.google.gson.GsonBuilder
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.android.qualifiers.ApplicationContext
import dagger.hilt.components.SingletonComponent
import okhttp3.OkHttpClient
import retrofit2.Retrofit
import retrofit2.converter.gson.GsonConverterFactory
import java.util.concurrent.TimeUnit
import javax.inject.Singleton

@Module
@InstallIn(SingletonComponent::class)
object AppModule {

    @Provides
    @Singleton
    fun provideAudioPlayer(
        @ApplicationContext context: Context,
        equalizerEngine: app.akroasis.audio.EqualizerEngine
    ): AudioPlayer {
        return AudioPlayer(context, equalizerEngine).apply {
            init()
        }
    }

    @Provides
    @Singleton
    fun providePlaybackQueue(): app.akroasis.audio.PlaybackQueue {
        return app.akroasis.audio.PlaybackQueue()
    }

    @Provides
    @Singleton
    fun provideAuthInterceptor(
        @ApplicationContext context: Context
    ): AuthInterceptor {
        return AuthInterceptor(context)
    }

    @Provides
    @Singleton
    fun provideGson(): Gson {
        return GsonBuilder()
            .setLenient()
            .create()
    }

    @Provides
    @Singleton
    fun provideOkHttpClient(
        authInterceptor: AuthInterceptor
    ): OkHttpClient {
        return OkHttpClient.Builder()
            .addInterceptor(authInterceptor)
            .connectTimeout(15, TimeUnit.SECONDS)
            .readTimeout(30, TimeUnit.SECONDS)
            .writeTimeout(30, TimeUnit.SECONDS)
            .build()
    }

    @Provides
    @Singleton
    fun provideRetrofit(
        okHttpClient: OkHttpClient,
        gson: Gson,
        serverPreferences: ServerPreferences
    ): Retrofit {
        return Retrofit.Builder()
            .baseUrl(serverPreferences.serverUrl)
            .client(okHttpClient)
            .addConverterFactory(GsonConverterFactory.create(gson))
            .build()
    }

    @Provides
    @Singleton
    fun provideMouseionApi(
        retrofit: Retrofit
    ): MouseionApi {
        return retrofit.create(MouseionApi::class.java)
    }

    @Provides
    @Singleton
    fun provideMusicDatabase(
        @ApplicationContext context: Context
    ): MusicDatabase {
        return Room.databaseBuilder(
            context,
            MusicDatabase::class.java,
            "akroasis_music_cache"
        ).build()
    }

    @Provides
    @Singleton
    fun provideMusicCacheDao(
        database: MusicDatabase
    ): app.akroasis.data.local.MusicCacheDao {
        return database.musicCacheDao()
    }
}

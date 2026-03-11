// Hilt module for audio dependencies
package app.akroasis.di

import app.akroasis.audio.AudioTrackFactory
import app.akroasis.audio.RealAudioTrackFactory
import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import javax.inject.Singleton

@Module
@InstallIn(SingletonComponent::class)
abstract class AudioModule {

    @Binds
    @Singleton
    abstract fun bindAudioTrackFactory(
        realAudioTrackFactory: RealAudioTrackFactory
    ): AudioTrackFactory
}

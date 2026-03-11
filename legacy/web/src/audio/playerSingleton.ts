// Singleton audio player — ONE instance shared across all components
import { WebAudioPlayer } from './WebAudioPlayer';

let instance: WebAudioPlayer | null = null;

export function getPlayer(): WebAudioPlayer {
  if (!instance) {
    instance = new WebAudioPlayer();
  }
  return instance;
}

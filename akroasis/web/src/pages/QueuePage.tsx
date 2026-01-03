// Queue management page with drag-to-reorder
import { usePlayerStore } from '../stores/playerStore';
import { useWebAudioPlayer } from '../hooks/useWebAudioPlayer';
import { Card } from '../components/Card';
import { Button } from '../components/Button';
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
} from '@dnd-kit/core';
import type { DragEndEvent } from '@dnd-kit/core';
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import type { Track } from '../types';

interface SortableTrackProps {
  track: Track;
  index: number;
  isCurrentTrack: boolean;
  onPlay: (track: Track) => void;
  onRemove: (index: number) => void;
}

function SortableTrack({ track, index, isCurrentTrack, onPlay, onRemove }: SortableTrackProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
  } = useSortable({ id: track.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  const formatTime = (ms: number) => {
    const totalSeconds = Math.floor(ms / 1000);
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;
    return `${minutes}:${seconds.toString().padStart(2, '0')}`;
  };

  const formatSize = (bytes: number) => {
    const mb = bytes / (1024 * 1024);
    return `${mb.toFixed(1)} MB`;
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={`flex items-center gap-3 p-3 rounded-lg transition-colors ${
        isCurrentTrack
          ? 'bg-bronze-700/50 border border-bronze-600'
          : 'bg-bronze-900/30 hover:bg-bronze-800/50'
      }`}
    >
      <button
        {...attributes}
        {...listeners}
        className="cursor-grab active:cursor-grabbing text-bronze-500 hover:text-bronze-300 p-1"
      >
        <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
          <path d="M7 2a2 2 0 1 0 .001 4.001A2 2 0 0 0 7 2zm0 6a2 2 0 1 0 .001 4.001A2 2 0 0 0 7 8zm0 6a2 2 0 1 0 .001 4.001A2 2 0 0 0 7 14zm6-8a2 2 0 1 0-.001-4.001A2 2 0 0 0 13 6zm0 2a2 2 0 1 0 .001 4.001A2 2 0 0 0 13 8zm0 6a2 2 0 1 0 .001 4.001A2 2 0 0 0 13 14z"/>
        </svg>
      </button>

      <div className="flex-1 min-w-0">
        <div className="flex items-baseline gap-2">
          <span className="text-bronze-500 text-sm font-mono">
            {(index + 1).toString().padStart(2, '0')}
          </span>
          <h3 className="text-bronze-100 font-medium truncate">{track.title}</h3>
          {isCurrentTrack && (
            <span className="text-xs text-bronze-400 bg-bronze-800 px-2 py-0.5 rounded">
              Now Playing
            </span>
          )}
        </div>
        <div className="flex items-center gap-2 mt-1 text-sm text-bronze-500">
          <span>{track.artist}</span>
          <span>•</span>
          <span>{formatTime(track.duration || 0)}</span>
          <span>•</span>
          <span className="uppercase">{track.format}</span>
          <span>•</span>
          <span>{formatSize(track.fileSize || 0)}</span>
        </div>
      </div>

      <button
        onClick={() => onPlay(track)}
        className="text-bronze-500 hover:text-bronze-300 p-2"
        title="Play now"
      >
        <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
          <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clipRule="evenodd"/>
        </svg>
      </button>

      <button
        onClick={() => onRemove(index)}
        className="text-bronze-600 hover:text-red-400 p-2"
        title="Remove from queue"
      >
        <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
          <path fillRule="evenodd" d="M9 2a1 1 0 00-.894.553L7.382 4H4a1 1 0 000 2v10a2 2 0 002 2h8a2 2 0 002-2V6a1 1 0 100-2h-3.382l-.724-1.447A1 1 0 0011 2H9zM7 8a1 1 0 012 0v6a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v6a1 1 0 102 0V8a1 1 0 00-1-1z" clipRule="evenodd"/>
        </svg>
      </button>
    </div>
  );
}

export function QueuePage() {
  const { queue, currentTrack, setQueue } = usePlayerStore();
  const { playTrack } = useWebAudioPlayer();

  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;

    if (over && active.id !== over.id) {
      const oldIndex = queue.findIndex((t) => t.id === active.id);
      const newIndex = queue.findIndex((t) => t.id === over.id);

      const newQueue = arrayMove(queue, oldIndex, newIndex);
      setQueue(newQueue);
    }
  };

  const handlePlay = (track: Track) => {
    playTrack(track);
  };

  const handleRemove = (index: number) => {
    const newQueue = queue.filter((_, i) => i !== index);
    setQueue(newQueue);
  };

  const handleClear = () => {
    if (confirm('Clear entire queue?')) {
      setQueue([]);
    }
  };

  const handleShuffle = () => {
    const shuffled = [...queue].sort(() => Math.random() - 0.5);
    setQueue(shuffled);
  };

  const totalDuration = queue.reduce((sum, track) => sum + (track.duration || 0), 0);
  const totalSize = queue.reduce((sum, track) => sum + (track.fileSize || 0), 0);

  const formatTime = (ms: number) => {
    const totalSeconds = Math.floor(ms / 1000);
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    }
    return `${minutes}m`;
  };

  const formatSize = (bytes: number) => {
    const gb = bytes / (1024 * 1024 * 1024);
    if (gb >= 1) {
      return `${gb.toFixed(2)} GB`;
    }
    const mb = bytes / (1024 * 1024);
    return `${mb.toFixed(1)} MB`;
  };

  return (
    <div className="container mx-auto p-4 max-w-4xl">
      <Card>
        <div className="flex items-center justify-between mb-6">
          <div>
            <h1 className="text-2xl font-bold text-bronze-100">Queue</h1>
            <p className="text-sm text-bronze-500 mt-1">
              {queue.length} tracks • {formatTime(totalDuration)} • {formatSize(totalSize)}
            </p>
          </div>
          <div className="flex gap-2">
            <Button
              variant="ghost"
              size="sm"
              onClick={handleShuffle}
              disabled={queue.length === 0}
            >
              <svg className="w-4 h-4 mr-2" fill="currentColor" viewBox="0 0 20 20">
                <path d="M4 3a2 2 0 100 4h12a2 2 0 100-4H4z"/>
                <path fillRule="evenodd" d="M3 8h14v7a2 2 0 01-2 2H5a2 2 0 01-2-2V8zm5 3a1 1 0 011-1h2a1 1 0 110 2H9a1 1 0 01-1-1z" clipRule="evenodd"/>
              </svg>
              Shuffle
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleClear}
              disabled={queue.length === 0}
            >
              <svg className="w-4 h-4 mr-2" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M9 2a1 1 0 00-.894.553L7.382 4H4a1 1 0 000 2v10a2 2 0 002 2h8a2 2 0 002-2V6a1 1 0 100-2h-3.382l-.724-1.447A1 1 0 0011 2H9zM7 8a1 1 0 012 0v6a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v6a1 1 0 102 0V8a1 1 0 00-1-1z" clipRule="evenodd"/>
              </svg>
              Clear
            </Button>
          </div>
        </div>

        {queue.length === 0 ? (
          <div className="text-center py-12">
            <svg className="w-16 h-16 text-bronze-700 mx-auto mb-4" fill="currentColor" viewBox="0 0 20 20">
              <path d="M9 2a1 1 0 000 2h2a1 1 0 100-2H9z"/>
              <path fillRule="evenodd" d="M4 5a2 2 0 012-2 3 3 0 003 3h2a3 3 0 003-3 2 2 0 012 2v11a2 2 0 01-2 2H6a2 2 0 01-2-2V5zm3 4a1 1 0 000 2h.01a1 1 0 100-2H7zm3 0a1 1 0 000 2h3a1 1 0 100-2h-3zm-3 4a1 1 0 100 2h.01a1 1 0 100-2H7zm3 0a1 1 0 100 2h3a1 1 0 100-2h-3z" clipRule="evenodd"/>
            </svg>
            <p className="text-bronze-500">Queue is empty</p>
            <p className="text-sm text-bronze-600 mt-1">
              Add tracks from the library to start playing
            </p>
          </div>
        ) : (
          <DndContext
            sensors={sensors}
            collisionDetection={closestCenter}
            onDragEnd={handleDragEnd}
          >
            <SortableContext
              items={queue.map((t) => t.id)}
              strategy={verticalListSortingStrategy}
            >
              <div className="space-y-2">
                {queue.map((track, index) => (
                  <SortableTrack
                    key={track.id}
                    track={track}
                    index={index}
                    isCurrentTrack={currentTrack?.id === track.id}
                    onPlay={handlePlay}
                    onRemove={handleRemove}
                  />
                ))}
              </div>
            </SortableContext>
          </DndContext>
        )}
      </Card>
    </div>
  );
}

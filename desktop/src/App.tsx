import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import Layout from "./components/Layout";
import Dsp from "./pages/Dsp";
import Settings from "./pages/Settings";
import AlbumsPage from "./features/library/AlbumsPage";
import TracksPage from "./features/library/TracksPage";
import AudiobooksPage from "./features/audiobook/pages/AudiobooksPage";
import AudiobookDetailPage from "./features/audiobook/pages/AudiobookDetailPage";
import AudiobookPlayerPage from "./features/audiobook/pages/AudiobookPlayerPage";
import PodcastsPage from "./features/podcast/pages/PodcastsPage";
import PodcastDetailPage from "./features/podcast/pages/PodcastDetailPage";
import EpisodeDetailPage from "./features/podcast/pages/EpisodeDetailPage";
import LatestEpisodesPage from "./features/podcast/pages/LatestEpisodesPage";
import DownloadQueuePage from "./features/podcast/pages/DownloadQueuePage";
import QueuePage from "./features/now-playing/pages/QueuePage";
import SignalPathPage from "./features/now-playing/pages/SignalPathPage";
import DashboardPage from "./features/management/pages/DashboardPage";
import MediaBrowsePage from "./features/management/pages/MediaBrowsePage";
import MediaDetailPage from "./features/management/pages/MediaDetailPage";
import MetadataEditPage from "./features/management/pages/MetadataEditPage";
import ManageDownloadQueuePage from "./features/management/pages/DownloadQueuePage";
import SearchPage from "./features/management/pages/SearchPage";
import RequestsPage from "./features/management/pages/RequestsPage";
import WantedPage from "./features/management/pages/WantedPage";
import IndexerSettingsPage from "./features/management/pages/IndexerSettingsPage";
import QualityProfilesPage from "./features/management/pages/QualityProfilesPage";
import LibraryHealthPage from "./features/management/pages/LibraryHealthPage";

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Layout />}>
          <Route index element={<Navigate to="/library/albums" replace />} />
          <Route path="library">
            <Route path="albums" element={<AlbumsPage />} />
            <Route path="tracks" element={<TracksPage />} />
            <Route path="audiobooks" element={<AudiobooksPage />} />
            <Route path="audiobooks/:id" element={<AudiobookDetailPage />} />
            <Route path="podcasts">
              <Route index element={<PodcastsPage />} />
              <Route path="latest" element={<LatestEpisodesPage />} />
              <Route path="downloads" element={<DownloadQueuePage />} />
              <Route path="episodes/:id" element={<EpisodeDetailPage />} />
              <Route path=":id" element={<PodcastDetailPage />} />
            </Route>
          </Route>
          <Route path="audiobook-player" element={<AudiobookPlayerPage />} />
          <Route path="dsp" element={<Dsp />} />
          <Route path="settings" element={<Settings />} />
          <Route path="queue" element={<QueuePage />} />
          <Route path="signal-path" element={<SignalPathPage />} />
          <Route path="manage">
            <Route index element={<DashboardPage />} />
            <Route path="media" element={<MediaBrowsePage />} />
            <Route path="media/:id" element={<MediaDetailPage />} />
            <Route path="media/:id/edit" element={<MetadataEditPage />} />
            <Route path="downloads" element={<ManageDownloadQueuePage />} />
            <Route path="search" element={<SearchPage />} />
            <Route path="requests" element={<RequestsPage />} />
            <Route path="wanted" element={<WantedPage />} />
            <Route path="indexers" element={<IndexerSettingsPage />} />
            <Route path="quality-profiles" element={<QualityProfilesPage />} />
            <Route path="health" element={<LibraryHealthPage />} />
          </Route>
        </Route>
      </Routes>
    </BrowserRouter>
  );
}

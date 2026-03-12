import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import Layout from "./components/Layout";
import Settings from "./pages/Settings";
import AlbumsPage from "./features/library/AlbumsPage";
import TracksPage from "./features/library/TracksPage";
import AudiobooksPage from "./features/library/AudiobooksPage";

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
          </Route>
          <Route path="settings" element={<Settings />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}

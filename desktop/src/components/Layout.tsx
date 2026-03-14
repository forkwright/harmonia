import { NavLink, Outlet } from "react-router-dom";
import NowPlayingBar from "../features/now-playing/components/NowPlayingBar";
import { usePositionSync } from "../features/audiobook/hooks/usePositionSync";
import AudiobookNowPlaying from "../features/audiobook/components/AudiobookNowPlaying";

const libraryItems = [
  { to: "/library/albums", label: "Albums" },
  { to: "/library/tracks", label: "Tracks" },
  { to: "/library/audiobooks", label: "Audiobooks" },
];

const podcastItems = [
  { to: "/library/podcasts", label: "Podcasts", end: true },
  { to: "/library/podcasts/latest", label: "Latest Episodes" },
  { to: "/library/podcasts/downloads", label: "Downloads" },
];

const managementItems = [
  { to: "/manage", label: "Dashboard", end: true },
  { to: "/manage/media", label: "Browse Media" },
  { to: "/manage/downloads", label: "Download Queue" },
  { to: "/manage/search", label: "Search" },
  { to: "/manage/requests", label: "Requests" },
  { to: "/manage/wanted", label: "Wanted" },
  { to: "/manage/indexers", label: "Indexers" },
  { to: "/manage/quality-profiles", label: "Quality Profiles" },
  { to: "/manage/health", label: "Library Health" },
];

function navLinkClass({ isActive }: { isActive: boolean }): string {
  return `block px-3 py-2 rounded-md text-sm font-medium transition-colors ${
    isActive
      ? "bg-gray-800 text-white"
      : "text-gray-400 hover:bg-gray-800 hover:text-white"
  }`;
}

function SidebarSection({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="pt-3">
      <p className="px-3 pt-1 pb-2 text-xs font-medium text-gray-500 uppercase tracking-wider">
        {title}
      </p>
      {children}
    </div>
  );
}

export default function Layout() {
  const { position } = usePositionSync();

  return (
    <div className="flex h-screen bg-gray-950 text-gray-100">
      <aside className="w-56 flex-shrink-0 bg-gray-900 flex flex-col">
        <div className="px-4 py-5 border-b border-gray-800">
          <span className="text-lg font-semibold tracking-wide">Harmonia</span>
        </div>
        <nav className="flex-1 px-2 py-4 space-y-1 overflow-y-auto">
          <p className="px-3 pt-1 pb-2 text-xs font-medium text-gray-500 uppercase tracking-wider">
            Music
          </p>
          {libraryItems.map(({ to, label }) => (
            <NavLink key={to} to={to} className={navLinkClass}>
              {label}
            </NavLink>
          ))}

          <SidebarSection title="Podcasts">
            {podcastItems.map(({ to, label, end }) => (
              <NavLink key={to} to={to} end={end} className={navLinkClass}>
                {label}
              </NavLink>
            ))}
          </SidebarSection>

          <SidebarSection title="Management">
            {managementItems.map(({ to, label, end }) => (
              <NavLink key={to} to={to} end={end} className={navLinkClass}>
                {label}
              </NavLink>
            ))}
          </SidebarSection>

          <div className="pt-3">
            <NavLink to="/dsp" className={navLinkClass}>
              DSP
            </NavLink>
            <NavLink to="/settings" className={navLinkClass}>
              Settings
            </NavLink>
          </div>
        </nav>
      </aside>
      <main className="flex-1 overflow-hidden flex flex-col">
        <div className="flex-1 overflow-hidden">
          <Outlet />
        </div>
        <div
          id="now-playing-bar"
          className="h-16 bg-gray-900 border-t border-gray-800 flex items-center px-4 flex-shrink-0"
        >
          {position ? (
            <AudiobookNowPlaying />
          ) : (
            <NowPlayingBar />
          )}
        </div>
      </main>
    </div>
  );
}

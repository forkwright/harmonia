import { NavLink, Outlet } from "react-router-dom";

const libraryItems = [
  { to: "/library/albums", label: "Albums" },
  { to: "/library/tracks", label: "Tracks" },
  { to: "/library/audiobooks", label: "Audiobooks" },
];

export default function Layout() {
  return (
    <div className="flex h-screen bg-gray-950 text-gray-100">
      <aside className="w-56 flex-shrink-0 bg-gray-900 flex flex-col">
        <div className="px-4 py-5 border-b border-gray-800">
          <span className="text-lg font-semibold tracking-wide">Harmonia</span>
        </div>
        <nav className="flex-1 px-2 py-4 space-y-1 overflow-y-auto">
          <p className="px-3 pt-1 pb-2 text-xs font-medium text-gray-500 uppercase tracking-wider">
            Library
          </p>
          {libraryItems.map(({ to, label }) => (
            <NavLink
              key={to}
              to={to}
              className={({ isActive }) =>
                `block px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                  isActive
                    ? "bg-gray-800 text-white"
                    : "text-gray-400 hover:bg-gray-800 hover:text-white"
                }`
              }
            >
              {label}
            </NavLink>
          ))}
          <div className="pt-3">
            <NavLink
              to="/dsp"
              className={({ isActive }) =>
                `block px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                  isActive
                    ? "bg-gray-800 text-white"
                    : "text-gray-400 hover:bg-gray-800 hover:text-white"
                }`
              }
            >
              DSP
            </NavLink>
            <NavLink
              to="/settings"
              className={({ isActive }) =>
                `block px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                  isActive
                    ? "bg-gray-800 text-white"
                    : "text-gray-400 hover:bg-gray-800 hover:text-white"
                }`
              }
            >
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
          <span className="text-sm text-gray-500">Now playing — coming in P3-11</span>
        </div>
      </main>
    </div>
  );
}

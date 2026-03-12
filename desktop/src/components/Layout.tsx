import { NavLink, Outlet } from "react-router-dom";

const navItems = [
  { to: "/", label: "Library", end: true },
  { to: "/settings", label: "Settings" },
];

export default function Layout() {
  return (
    <div className="flex h-screen bg-gray-950 text-gray-100">
      <aside className="w-56 flex-shrink-0 bg-gray-900 flex flex-col">
        <div className="px-4 py-5 border-b border-gray-800">
          <span className="text-lg font-semibold tracking-wide">Harmonia</span>
        </div>
        <nav className="flex-1 px-2 py-4 space-y-1">
          {navItems.map(({ to, label, end }) => (
            <NavLink
              key={to}
              to={to}
              end={end}
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
        </nav>
      </aside>
      <main className="flex-1 overflow-auto">
        <Outlet />
      </main>
      <div
        id="now-playing-bar"
        className="absolute bottom-0 left-56 right-0 h-16 bg-gray-900 border-t border-gray-800 flex items-center px-4"
      >
        <span className="text-sm text-gray-500">Now playing — coming in P3-11</span>
      </div>
    </div>
  );
}

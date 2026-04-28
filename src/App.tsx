import { Outlet, NavLink } from "react-router-dom"
import { cn } from "@/lib/utils"

const navItems = [
  { to: "/library", label: "Library" },
  { to: "/stats",   label: "Stats" },
  { to: "/import",  label: "Import" },
]

export default function App() {
  return (
    <div className="min-h-screen flex bg-background text-foreground">
      <aside className="w-56 border-r border-border p-4 flex flex-col">
        <div className="px-2 mb-6">
          <h1 className="text-lg font-semibold tracking-tight">Memex</h1>
          <p className="text-xs text-muted-foreground mt-0.5">Local AI history</p>
        </div>
        <nav className="space-y-1">
          {navItems.map(({ to, label }) => (
            <NavLink
              key={to}
              to={to}
              className={({ isActive }) =>
                cn(
                  "block px-3 py-1.5 rounded-md text-sm transition-colors",
                  isActive
                    ? "bg-primary text-primary-foreground"
                    : "text-muted-foreground hover:bg-accent hover:text-accent-foreground"
                )
              }
            >
              {label}
            </NavLink>
          ))}
        </nav>
        <div className="mt-auto px-3 text-[11px] text-muted-foreground">
          v0.1 · 100% local
        </div>
      </aside>
      <main className="flex-1 overflow-auto">
        <Outlet />
      </main>
    </div>
  )
}

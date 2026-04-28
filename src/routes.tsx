import { createBrowserRouter, Navigate } from "react-router-dom"
import App from "./App"
import Onboarding from "./pages/Onboarding"
import Library from "./pages/Library"
import Conversation from "./pages/Conversation"
import Stats from "./pages/Stats"

export const router = createBrowserRouter([
  {
    path: "/",
    element: <App />,
    children: [
      { index: true, element: <Navigate to="/library" replace /> },
      { path: "import", element: <Onboarding /> },
      { path: "library", element: <Library /> },
      { path: "conversation/:id", element: <Conversation /> },
      { path: "stats", element: <Stats /> },
    ],
  },
])

import { BrowserRouter as Router, Routes, Route, Link } from "react-router-dom";
import { Dashboard } from "./components/Dashboard";
import { Settings } from "./components/Settings";
import { Logs } from "./components/Logs";
import { LayoutDashboard, Settings as SettingsIcon, FileText } from "lucide-react";

function App() {
  return (
    <Router>
      <div className="flex h-screen bg-gray-100">
        <aside className="w-64 bg-white border-r p-4">
          <div className="text-xl font-bold mb-6 px-4 text-blue-600">Claw9Router</div>
          <nav className="space-y-2">
            <Link to="/" className="flex items-center gap-2 px-4 py-2 hover:bg-gray-100 rounded text-gray-700">
              <LayoutDashboard className="h-4 w-4" /> Dashboard
            </Link>
            <Link to="/settings" className="flex items-center gap-2 px-4 py-2 hover:bg-gray-100 rounded text-gray-700">
              <SettingsIcon className="h-4 w-4" /> Settings
            </Link>
            <Link to="/logs" className="flex items-center gap-2 px-4 py-2 hover:bg-gray-100 rounded text-gray-700">
              <FileText className="h-4 w-4" /> Logs
            </Link>
          </nav>
        </aside>
        <main className="flex-1 p-8 overflow-auto">
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/settings" element={<Settings />} />
            <Route path="/logs" element={<Logs />} />
          </Routes>
        </main>
      </div>
    </Router>
  );
}

export default App;

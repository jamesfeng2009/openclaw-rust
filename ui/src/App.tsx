import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { Layout } from 'lucide-react'
import { Sidebar } from './components/Sidebar'
import { ChatView } from './components/ChatView'
import { AgentPanel } from './components/AgentPanel'
import { SettingsPage } from './pages/SettingsPage'
import { ChannelsPage } from './pages/ChannelsPage'
import { SessionsPage } from './pages/SessionsPage'
import { useAppStore } from './store/appStore'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60,
      refetchOnWindowFocus: false,
    },
  },
})

function AppLayout() {
  const { sidebarOpen, setSidebarOpen } = useAppStore()

  return (
    <div className="flex h-screen bg-gray-50 dark:bg-gray-900">
      <Sidebar />
      
      <div className="flex-1 flex flex-col overflow-hidden">
        {/* Header */}
        <header className="h-14 bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 flex items-center px-4 justify-between">
          <div className="flex items-center gap-3">
            <button
              onClick={() => setSidebarOpen(!sidebarOpen)}
              className="p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg"
            >
              <Layout className="w-5 h-5" />
            </button>
            <h1 className="text-lg font-semibold">OpenClaw</h1>
          </div>
          
          <div className="flex items-center gap-2">
            <div className="flex items-center gap-2 px-3 py-1 bg-green-100 dark:bg-green-900/30 rounded-full">
              <span className="w-2 h-2 bg-green-500 rounded-full animate-pulse" />
              <span className="text-sm text-green-700 dark:text-green-400">Online</span>
            </div>
          </div>
        </header>

        {/* Main Content */}
        <main className="flex-1 flex overflow-hidden">
          <ChatView />
          <AgentPanel />
        </main>
      </div>
    </div>
  )
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<AppLayout />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="/channels" element={<ChannelsPage />} />
          <Route path="/sessions" element={<SessionsPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </BrowserRouter>
    </QueryClientProvider>
  )
}

export default App

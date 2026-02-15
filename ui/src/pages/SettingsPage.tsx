import { useState } from 'react'
import { Settings as SettingsIcon, Key, Globe, Bell, Palette } from 'lucide-react'
import { useAppStore } from '../store/appStore'

export function SettingsPage() {
  const { darkMode, setDarkMode, gatewayUrl, setGatewayUrl } = useAppStore()
  const [apiKeys, setApiKeys] = useState<Record<string, string>>({})

  const sections = [
    { id: 'general', icon: SettingsIcon, label: '通用' },
    { id: 'api', icon: Key, label: 'API 密钥' },
    { id: 'gateway', icon: Globe, label: '网关' },
    { id: 'notifications', icon: Bell, label: '通知' },
    { id: 'appearance', icon: Palette, label: '外观' },
  ]

  return (
    <div className="flex h-screen bg-gray-50 dark:bg-gray-900">
      {/* Sidebar */}
      <aside className="w-64 bg-white dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700 p-4">
        <h2 className="text-lg font-semibold mb-4">设置</h2>
        <nav className="space-y-1">
          {sections.map((section) => (
            <button
              key={section.id}
              className="w-full flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700"
            >
              <section.icon className="w-5 h-5" />
              <span>{section.label}</span>
            </button>
          ))}
        </nav>
      </aside>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-8">
        <div className="max-w-2xl mx-auto space-y-8">
          {/* Gateway URL */}
          <section>
            <h3 className="text-lg font-semibold mb-4">网关设置</h3>
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium mb-2">Gateway URL</label>
                <input
                  type="text"
                  value={gatewayUrl}
                  onChange={(e) => setGatewayUrl(e.target.value)}
                  className="w-full px-4 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700"
                  placeholder="http://localhost:18789"
                />
              </div>
            </div>
          </section>

          {/* Appearance */}
          <section>
            <h3 className="text-lg font-semibold mb-4">外观</h3>
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <span>深色模式</span>
                <button
                  onClick={() => setDarkMode(!darkMode)}
                  className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                    darkMode ? 'bg-primary-600' : 'bg-gray-300'
                  }`}
                >
                  <span
                    className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                      darkMode ? 'translate-x-6' : 'translate-x-1'
                    }`}
                  />
                </button>
              </div>
            </div>
          </section>

          {/* API Keys */}
          <section>
            <h3 className="text-lg font-semibold mb-4">API 密钥</h3>
            <div className="space-y-4">
              {['OpenAI', 'Anthropic', 'Google'].map((provider) => (
                <div key={provider}>
                  <label className="block text-sm font-medium mb-2">{provider}</label>
                  <input
                    type="password"
                    value={apiKeys[provider] || ''}
                    onChange={(e) => setApiKeys({ ...apiKeys, [provider]: e.target.value })}
                    className="w-full px-4 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700"
                    placeholder="sk-..."
                  />
                </div>
              ))}
            </div>
          </section>
        </div>
      </div>
    </div>
  )
}

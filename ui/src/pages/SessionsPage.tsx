import { useState, useEffect } from 'react'
import { Users, Plus, Trash2, RefreshCw, MessageSquare } from 'lucide-react'
import { useAppStore } from '../store/appStore'

export function SessionsPage() {
  const { sessions, setSessions, currentSessionId, setCurrentSession, gatewayUrl } = useAppStore()
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    fetchSessions()
  }, [])

  const fetchSessions = async () => {
    setLoading(true)
    try {
      const response = await fetch(`${gatewayUrl}/api/sessions`)
      if (response.ok) {
        const data = await response.json()
        setSessions(data.sessions || [])
      }
    } catch (error) {
      console.error('Failed to fetch sessions:', error)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="flex h-screen bg-gray-50 dark:bg-gray-900">
      {/* Sidebar */}
      <aside className="w-64 bg-white dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700 p-4">
        <h2 className="text-lg font-semibold mb-4">会话管理</h2>
        <nav className="space-y-1">
          <button
            onClick={() => window.history.back()}
            className="w-full flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700"
          >
            <RefreshCw className="w-5 h-5" />
            <span>返回</span>
          </button>
        </nav>
      </aside>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-8">
        <div className="max-w-4xl mx-auto">
          <div className="flex items-center justify-between mb-6">
            <h1 className="text-2xl font-bold">会话管理</h1>
            <button className="flex items-center gap-2 px-4 py-2 bg-primary-500 text-white rounded-lg hover:bg-primary-600">
              <Plus className="w-5 h-5" />
              新建会话
            </button>
          </div>

          {/* Sessions List */}
          <section>
            <h2 className="text-lg font-semibold mb-4">活跃会话</h2>
            <div className="space-y-2">
              {loading ? (
                <div className="text-center py-8 text-gray-500">加载中...</div>
              ) : sessions.length === 0 ? (
                <div className="text-center py-8 text-gray-500">
                  <Users className="w-10 h-10 mx-auto mb-2 opacity-50" />
                  <p>暂无活跃会话</p>
                  <p className="text-sm mt-1">开始对话将自动创建会话</p>
                </div>
              ) : (
                sessions.map((session) => (
                  <div
                    key={session.id}
                    onClick={() => setCurrentSession(session.id)}
                    className={`flex items-center justify-between p-4 bg-white dark:bg-gray-800 rounded-lg border cursor-pointer transition-colors ${
                      currentSessionId === session.id
                        ? 'border-primary-500 ring-2 ring-primary-200'
                        : 'border-gray-200 dark:border-gray-700 hover:border-gray-300'
                    }`}
                  >
                    <div className="flex items-center gap-3">
                      <MessageSquare className="w-5 h-5 text-gray-400" />
                      <div>
                        <p className="font-medium">{session.name}</p>
                        <p className="text-sm text-gray-500">
                          {session.channelId && `通道: ${session.channelId}`}
                          {session.agentId && ` | Agent: ${session.agentId}`}
                        </p>
                      </div>
                    </div>
                    <button
                      onClick={(e) => {
                        e.stopPropagation()
                      }}
                      className="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
                    >
                      <Trash2 className="w-4 h-4 text-red-500" />
                    </button>
                  </div>
                ))
              )}
            </div>
          </section>

          {/* Session Stats */}
          <section className="mt-8">
            <h2 className="text-lg font-semibold mb-4">会话统计</h2>
            <div className="grid grid-cols-3 gap-4">
              <div className="p-4 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
                <p className="text-2xl font-bold text-primary-600">{sessions.length}</p>
                <p className="text-sm text-gray-500">总会话数</p>
              </div>
              <div className="p-4 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
                <p className="text-2xl font-bold text-green-600">
                  {sessions.filter((s) => s.id === currentSessionId).length || 0}
                </p>
                <p className="text-sm text-gray-500">活跃会话</p>
              </div>
              <div className="p-4 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
                <p className="text-2xl font-bold text-gray-600">0</p>
                <p className="text-sm text-gray-500">今日消息</p>
              </div>
            </div>
          </section>
        </div>
      </div>
    </div>
  )
}

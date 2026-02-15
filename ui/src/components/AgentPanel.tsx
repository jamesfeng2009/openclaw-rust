import { useState } from 'react'
import { Bot, ChevronRight, Circle, Plus } from 'lucide-react'
import { useAppStore, type Agent } from '../store/appStore'

export function AgentPanel() {
  const { agents } = useAppStore()
  const [selectedAgent, setSelectedAgent] = useState<Agent | null>(null)

  const getStatusColor = (status: Agent['status']) => {
    switch (status) {
      case 'online': return 'text-green-500'
      case 'idle': return 'text-yellow-500'
      case 'offline': return 'text-gray-400'
    }
  }

  const getStatusText = (status: Agent['status']) => {
    switch (status) {
      case 'online': return '在线'
      case 'idle': return '空闲'
      case 'offline': return '离线'
    }
  }

  return (
    <div className="w-72 bg-gray-50 dark:bg-gray-900 border-l border-gray-200 dark:border-gray-700 flex flex-col">
      {/* Header */}
      <div className="h-14 px-4 flex items-center justify-between border-b border-gray-200 dark:border-gray-700">
        <h2 className="font-semibold">智能体</h2>
        <button className="p-1 hover:bg-gray-200 dark:hover:bg-gray-700 rounded">
          <Plus className="w-5 h-5" />
        </button>
      </div>

      {/* Agent List */}
      <div className="flex-1 overflow-y-auto p-2">
        {agents.length === 0 ? (
          <div className="text-center text-gray-400 py-8">
            <Bot className="w-10 h-10 mx-auto mb-2 opacity-50" />
            <p className="text-sm">暂无智能体</p>
          </div>
        ) : (
          agents.map((agent) => (
            <button
              key={agent.id}
              onClick={() => setSelectedAgent(agent)}
              className={`w-full flex items-center gap-3 p-3 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors ${
                selectedAgent?.id === agent.id ? 'bg-gray-200 dark:bg-gray-700' : ''
              }`}
            >
              <div className="w-10 h-10 rounded-full bg-primary-100 dark:bg-primary-900 flex items-center justify-center">
                <Bot className="w-5 h-5 text-primary-600" />
              </div>
              <div className="flex-1 text-left">
                <p className="font-medium text-sm">{agent.name}</p>
                <div className="flex items-center gap-1 text-xs text-gray-500">
                  <Circle className={`w-2 h-2 fill-current ${getStatusColor(agent.status)}`} />
                  <span>{getStatusText(agent.status)}</span>
                </div>
              </div>
              <ChevronRight className="w-4 h-4 text-gray-400" />
            </button>
          ))
        )}
      </div>

      {/* Selected Agent Details */}
      {selectedAgent && (
        <div className="p-4 border-t border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
          <h3 className="font-semibold mb-3">{selectedAgent.name}</h3>
          <div className="space-y-2 text-sm">
            <div className="flex justify-between">
              <span className="text-gray-500">状态</span>
              <span className={getStatusColor(selectedAgent.status)}>
                {getStatusText(selectedAgent.status)}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-500">ID</span>
              <span className="text-xs font-mono">{selectedAgent.id}</span>
            </div>
          </div>
          <div className="mt-4 flex gap-2">
            <button className="flex-1 px-3 py-1.5 text-sm bg-primary-500 text-white rounded-lg hover:bg-primary-600">
              配置
            </button>
            <button className="flex-1 px-3 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700">
              统计
            </button>
          </div>
        </div>
      )}
    </div>
  )
}

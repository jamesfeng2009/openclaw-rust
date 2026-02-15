import { useState } from 'react'

export interface DeviceNode {
  id: string
  name: string
  type: 'camera' | 'screen' | 'location' | 'notification' | 'system'
  enabled: boolean
  available: boolean
  capabilities: DeviceCapability[]
}

export interface DeviceCapability {
  id: string
  name: string
  description: string
  enabled: boolean
}

export function DevicesPage() {
  const [nodes, setNodes] = useState<DeviceNode[]>([
    {
      id: 'camera',
      name: '相机',
      type: 'camera',
      enabled: true,
      available: true,
      capabilities: [
        { id: 'camera.snap', name: '相机拍照', description: '使用设备相机拍摄照片', enabled: true },
        { id: 'camera.clip', name: '相机录像', description: '使用设备相机录制视频', enabled: true },
      ],
    },
    {
      id: 'screen',
      name: '屏幕录制',
      type: 'screen',
      enabled: true,
      available: true,
      capabilities: [
        { id: 'screen.record', name: '屏幕录制', description: '录制屏幕内容', enabled: true },
        { id: 'screen.screenshot', name: '屏幕截图', description: '截取屏幕内容', enabled: true },
      ],
    },
    {
      id: 'location',
      name: '定位',
      type: 'location',
      enabled: true,
      available: true,
      capabilities: [
        { id: 'location.get', name: '获取定位', description: '获取设备当前地理位置', enabled: true },
      ],
    },
    {
      id: 'notification',
      name: '通知推送',
      type: 'notification',
      enabled: true,
      available: true,
      capabilities: [
        { id: 'notification.send', name: '发送通知', description: '向设备发送通知', enabled: true },
      ],
    },
    {
      id: 'system',
      name: '系统命令',
      type: 'system',
      enabled: true,
      available: true,
      capabilities: [
        { id: 'system.run', name: '执行命令', description: '在设备上执行系统命令', enabled: true },
        { id: 'system.notify', name: '系统通知', description: '发送系统级通知', enabled: true },
      ],
    },
  ])

  const [activeNode, setActiveNode] = useState<string | null>(null)
  const [logs, setLogs] = useState<string[]>([])

  const toggleNode = (nodeId: string) => {
    setNodes(prev => prev.map(node => 
      node.id === nodeId ? { ...node, enabled: !node.enabled } : node
    ))
  }

  const toggleCapability = (nodeId: string, capId: string) => {
    setNodes(prev => prev.map(node => 
      node.id === nodeId 
        ? { 
            ...node, 
            capabilities: node.capabilities.map(cap => 
              cap.id === capId ? { ...cap, enabled: !cap.enabled } : cap
            )
          } 
        : node
    ))
  }

  const executeCapability = async (_nodeId: string, capId: string) => {
    const log = `[${new Date().toLocaleTimeString()}] 执行 ${capId}...`
    setLogs(prev => [...prev, log])
    
    setTimeout(() => {
      setLogs(prev => [...prev, `[${new Date().toLocaleTimeString()}] ${capId} 执行完成`])
    }, 1000)
  }

  const getNodeIcon = (type: DeviceNode['type']) => {
    switch (type) {
      case 'camera': return '📷'
      case 'screen': return '🖥️'
      case 'location': return '📍'
      case 'notification': return '🔔'
      case 'system': return '⚙️'
    }
  }

  return (
    <div className="flex h-full">
      <div className="w-80 border-r border-gray-200 dark:border-gray-700 p-4 overflow-y-auto">
        <h2 className="text-lg font-semibold mb-4">设备节点</h2>
        <div className="space-y-3">
          {nodes.map(node => (
            <div
              key={node.id}
              className={`p-3 rounded-lg border ${
                activeNode === node.id 
                  ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20' 
                  : 'border-gray-200 dark:border-gray-700'
              }`}
            >
              <div className="flex items-center justify-between">
                <button
                  onClick={() => setActiveNode(activeNode === node.id ? null : node.id)}
                  className="flex items-center gap-2"
                >
                  <span className="text-xl">{getNodeIcon(node.type)}</span>
                  <span className="font-medium">{node.name}</span>
                </button>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input
                    type="checkbox"
                    checked={node.enabled}
                    onChange={() => toggleNode(node.id)}
                    className="sr-only peer"
                  />
                  <div className="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                </label>
              </div>
              {node.available && (
                <span className="text-xs text-green-600 dark:text-green-400">可用</span>
              )}
              {!node.available && (
                <span className="text-xs text-red-600 dark:text-red-400">不可用</span>
              )}
            </div>
          ))}
        </div>
      </div>

      <div className="flex-1 p-6 overflow-y-auto">
        {activeNode ? (
          <div>
            <div className="flex items-center justify-between mb-6">
              <h2 className="text-xl font-semibold">
                {getNodeIcon(nodes.find(n => n.id === activeNode)?.type || 'system')} {nodes.find(n => n.id === activeNode)?.name}
              </h2>
            </div>

            <div className="mb-6">
              <h3 className="text-sm font-medium text-gray-500 dark:text-gray-400 mb-3">能力</h3>
              <div className="grid gap-3">
                {nodes.find(n => n.id === activeNode)?.capabilities.map(cap => (
                  <div
                    key={cap.id}
                    className="flex items-center justify-between p-3 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700"
                  >
                    <div className="flex items-center gap-3">
                      <input
                        type="checkbox"
                        checked={cap.enabled}
                        onChange={() => toggleCapability(activeNode, cap.id)}
                        className="w-4 h-4 text-blue-600 rounded border-gray-300 focus:ring-blue-500"
                      />
                      <div>
                        <div className="font-medium">{cap.name}</div>
                        <div className="text-sm text-gray-500">{cap.description}</div>
                      </div>
                    </div>
                    <button
                      onClick={() => executeCapability(activeNode, cap.id)}
                      className="px-3 py-1 text-sm bg-blue-500 text-white rounded hover:bg-blue-600"
                    >
                      执行
                    </button>
                  </div>
                ))}
              </div>
            </div>

            <div>
              <h3 className="text-sm font-medium text-gray-500 dark:text-gray-400 mb-3">执行日志</h3>
              <div className="bg-gray-900 text-gray-100 p-4 rounded-lg h-48 overflow-y-auto font-mono text-sm">
                {logs.length === 0 ? (
                  <span className="text-gray-500">等待执行...</span>
                ) : (
                  logs.map((log, i) => (
                    <div key={i}>{log}</div>
                  ))
                )}
              </div>
            </div>
          </div>
        ) : (
          <div className="flex items-center justify-center h-full text-gray-500">
            选择一个设备节点进行配置
          </div>
        )}
      </div>
    </div>
  )
}

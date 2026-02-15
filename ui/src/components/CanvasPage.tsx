import { useState, useEffect, useRef, useCallback } from 'react'
import { useAppStore } from '../store/appStore'

export interface CanvasElement {
  id: string
  type: 'rect' | 'circle' | 'line' | 'text' | 'image' | 'path'
  x: number
  y: number
  width?: number
  height?: number
  radius?: number
  color: string
  strokeWidth?: number
  text?: string
  fontSize?: number
  src?: string
  points?: { x: number; y: number }[]
}

export interface CanvasUser {
  id: string
  name: string
  color: string
  cursor: { x: number; y: number } | null
}

export function CanvasPage() {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)
  const wsRef = useRef<WebSocket | null>(null)
  
  const [canvasState, setCanvasState] = useState({
    id: '',
    name: '新画布',
    elements: [] as CanvasElement[],
    users: [] as CanvasUser[],
    selectedTool: 'select' as 'select' | 'rect' | 'circle' | 'line' | 'text' | 'pen' | 'eraser',
    strokeColor: '#000000',
    fillColor: '#ffffff',
    strokeWidth: 2,
  })
  const [isDrawing, setIsDrawing] = useState(false)
  const [currentPath, setCurrentPath] = useState<{ x: number; y: number }[]>([])
  const [startPoint, setStartPoint] = useState<{ x: number; y: number } | null>(null)
  const [connectedUsers, setConnectedUsers] = useState(0)
  
  const gatewayUrl = useAppStore(state => state.gatewayUrl)

  const connectWebSocket = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return
    
    const wsUrl = gatewayUrl.replace('http', 'ws') + '/ws'
    const ws = new WebSocket(wsUrl)
    
    ws.onopen = () => {
      console.log('Canvas WebSocket connected')
    }
    
    ws.onmessage = (event: MessageEvent) => {
      try {
        const message = JSON.parse(event.data)
        if (message.type === 'canvas_update') {
          setCanvasState(prev => ({
            ...prev,
            elements: message.payload.elements || [],
          }))
        } else if (message.type === 'cursor_move') {
          setCanvasState(prev => ({
            ...prev,
            users: prev.users.map(u => 
              u.id === message.payload.userId 
                ? { ...u, cursor: message.payload.cursor }
                : u
            ).concat(
              prev.users.find(u => u.id === message.payload.userId) 
                ? [] 
                : [{ id: message.payload.userId, name: message.payload.name, color: message.payload.color, cursor: message.payload.cursor }]
            )
          }))
        } else if (message.type === 'users_count') {
          setConnectedUsers(message.payload.count)
        }
      } catch (e) {
        console.error('Failed to parse canvas message:', e)
      }
    }
    
    ws.onclose = () => {
      console.log('Canvas WebSocket disconnected, reconnecting...')
      setTimeout(connectWebSocket, 3000)
    }
    
    ws.onerror = (error) => {
      console.error('Canvas WebSocket error:', error)
    }
    
    wsRef.current = ws
  }, [gatewayUrl])

  useEffect(() => {
    connectWebSocket()
    return () => {
      wsRef.current?.close()
    }
  }, [connectWebSocket])

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return

    const resizeCanvas = () => {
      const container = containerRef.current
      if (container) {
        canvas.width = container.clientWidth
        canvas.height = container.clientHeight
        drawCanvas()
      }
    }

    resizeCanvas()
    window.addEventListener('resize', resizeCanvas)
    return () => window.removeEventListener('resize', resizeCanvas)
  }, [])

  const drawCanvas = useCallback(() => {
    const canvas = canvasRef.current
    if (!canvas) return

    const ctx = canvas.getContext('2d')
    if (!ctx) return

    ctx.clearRect(0, 0, canvas.width, canvas.height)
    ctx.fillStyle = '#ffffff'
    ctx.fillRect(0, 0, canvas.width, canvas.height)

    canvasState.elements.forEach(element => {
      ctx.beginPath()
      ctx.strokeStyle = element.color
      ctx.lineWidth = element.strokeWidth || 2

      switch (element.type) {
        case 'rect':
          ctx.fillStyle = element.color
          ctx.fillRect(element.x, element.y, element.width || 100, element.height || 100)
          break
        case 'circle':
          ctx.beginPath()
          ctx.arc(element.x, element.y, element.radius || 50, 0, Math.PI * 2)
          ctx.stroke()
          break
        case 'line':
          ctx.beginPath()
          ctx.moveTo(element.x, element.y)
          ctx.lineTo(element.x + (element.width || 0), element.y + (element.height || 0))
          ctx.stroke()
          break
        case 'text':
          ctx.font = `${element.fontSize || 16}px Arial`
          ctx.fillStyle = element.color
          ctx.fillText(element.text || '', element.x, element.y)
          break
        case 'path':
          if (element.points && element.points.length > 0) {
            ctx.beginPath()
            ctx.moveTo(element.points[0].x, element.points[0].y)
            element.points.forEach(point => {
              ctx.lineTo(point.x, point.y)
            })
            ctx.stroke()
          }
          break
      }
    })

    canvasState.users.forEach(user => {
      if (user.cursor) {
        ctx.beginPath()
        ctx.fillStyle = user.color
        ctx.arc(user.cursor.x, user.cursor.y, 5, 0, Math.PI * 2)
        ctx.fill()
        ctx.font = '12px Arial'
        ctx.fillText(user.name, user.cursor.x + 8, user.cursor.y - 8)
      }
    })

    if (isDrawing && currentPath.length > 0) {
      ctx.beginPath()
      ctx.strokeStyle = canvasState.strokeColor
      ctx.lineWidth = canvasState.strokeWidth
      ctx.moveTo(currentPath[0].x, currentPath[0].y)
      currentPath.forEach(point => {
        ctx.lineTo(point.x, point.y)
      })
      ctx.stroke()
    }
  }, [canvasState, isDrawing, currentPath])

  useEffect(() => {
    drawCanvas()
  }, [drawCanvas])

  const getMousePos = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current
    if (!canvas) return { x: 0, y: 0 }
    const rect = canvas.getBoundingClientRect()
    return {
      x: e.clientX - rect.left,
      y: e.clientY - rect.top
    }
  }

  const handleMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const pos = getMousePos(e)
    setIsDrawing(true)
    setStartPoint(pos)
    
    if (canvasState.selectedTool === 'pen' || canvasState.selectedTool === 'eraser') {
      setCurrentPath([pos])
    }
  }

  const handleMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const pos = getMousePos(e)

    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({
        type: 'cursor_move',
        payload: { cursor: pos }
      }))
    }

    if (!isDrawing) return

    if (canvasState.selectedTool === 'pen' || canvasState.selectedTool === 'eraser') {
      setCurrentPath(prev => [...prev, pos])
    }
  }

  const handleMouseUp = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!isDrawing) return
    const pos = getMousePos(e)

    let newElement: CanvasElement | null = null

    switch (canvasState.selectedTool) {
      case 'rect':
        newElement = {
          id: Date.now().toString(),
          type: 'rect',
          x: Math.min(startPoint?.x || 0, pos.x),
          y: Math.min(startPoint?.y || 0, pos.y),
          width: Math.abs((pos.x - (startPoint?.x || 0))),
          height: Math.abs((pos.y - (startPoint?.y || 0))),
          color: canvasState.fillColor,
          strokeWidth: canvasState.strokeWidth,
        }
        break
      case 'circle':
        const radius = Math.sqrt(
          Math.pow(pos.x - (startPoint?.x || 0), 2) + 
          Math.pow(pos.y - (startPoint?.y || 0), 2)
        )
        newElement = {
          id: Date.now().toString(),
          type: 'circle',
          x: startPoint?.x || 0,
          y: startPoint?.y || 0,
          radius,
          color: canvasState.strokeColor,
          strokeWidth: canvasState.strokeWidth,
        }
        break
      case 'line':
        newElement = {
          id: Date.now().toString(),
          type: 'line',
          x: startPoint?.x || 0,
          y: startPoint?.y || 0,
          width: pos.x - (startPoint?.x || 0),
          height: pos.y - (startPoint?.y || 0),
          color: canvasState.strokeColor,
          strokeWidth: canvasState.strokeWidth,
        }
        break
      case 'pen':
        if (currentPath.length > 1) {
          newElement = {
            id: Date.now().toString(),
            type: 'path',
            x: 0,
            y: 0,
            color: canvasState.strokeColor,
            strokeWidth: canvasState.strokeWidth,
            points: currentPath,
          }
        }
        break
      case 'eraser':
        const eraseRadius = 20
        setCanvasState(prev => ({
          ...prev,
          elements: prev.elements.filter(el => {
            const dist = Math.sqrt(
              Math.pow((el.x || 0) - pos.x, 2) + 
              Math.pow((el.y || 0) - pos.y, 2)
            )
            return dist > eraseRadius
          })
        }))
        break
    }

    if (newElement) {
      const newElements = [...canvasState.elements, newElement]
      setCanvasState(prev => ({ ...prev, elements: newElements }))
      
      if (wsRef.current?.readyState === WebSocket.OPEN) {
        wsRef.current.send(JSON.stringify({
          type: 'canvas_update',
          payload: { elements: newElements }
        }))
      }
    }

    setIsDrawing(false)
    setCurrentPath([])
    setStartPoint(null)
  }

  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if (e.key === 'Delete' || e.key === 'Backspace') {
      setCanvasState(prev => ({ ...prev, elements: [] }))
      if (wsRef.current?.readyState === WebSocket.OPEN) {
        wsRef.current.send(JSON.stringify({
          type: 'canvas_update',
          payload: { elements: [] }
        }))
      }
    }
  }, [])

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [handleKeyDown])

  const tools = [
    { id: 'select', label: '选择', icon: '↖' },
    { id: 'rect', label: '矩形', icon: '▢' },
    { id: 'circle', label: '圆形', icon: '○' },
    { id: 'line', label: '直线', icon: '/' },
    { id: 'pen', label: '画笔', icon: '✏' },
    { id: 'eraser', label: '橡皮', icon: '◻' },
  ] as const

  const colors = ['#000000', '#ff0000', '#00ff00', '#0000ff', '#ffff00', '#ff00ff', '#00ffff', '#ffffff']

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-2 p-2 bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
        <div className="flex gap-1">
          {tools.map(tool => (
            <button
              key={tool.id}
              onClick={() => setCanvasState(prev => ({ ...prev, selectedTool: tool.id }))}
              className={`p-2 rounded ${
                canvasState.selectedTool === tool.id
                  ? 'bg-blue-500 text-white'
                  : 'hover:bg-gray-100 dark:hover:bg-gray-700'
              }`}
              title={tool.label}
            >
              {tool.icon}
            </button>
          ))}
        </div>
        
        <div className="h-6 w-px bg-gray-300 dark:bg-gray-600" />
        
        <div className="flex gap-1">
          {colors.map(color => (
            <button
              key={color}
              onClick={() => setCanvasState(prev => ({ 
                ...prev, 
                strokeColor: color,
                fillColor: color 
              }))}
              className={`w-6 h-6 rounded border-2 ${
                canvasState.strokeColor === color ? 'border-blue-500' : 'border-gray-300'
              }`}
              style={{ backgroundColor: color }}
            />
          ))}
        </div>
        
        <div className="h-6 w-px bg-gray-300 dark:bg-gray-600" />
        
        <div className="flex items-center gap-2">
          <label className="text-sm">线宽:</label>
          <input
            type="range"
            min="1"
            max="20"
            value={canvasState.strokeWidth}
            onChange={(e) => setCanvasState(prev => ({ 
              ...prev, 
              strokeWidth: parseInt(e.target.value) 
            }))}
            className="w-24"
          />
          <span className="text-sm">{canvasState.strokeWidth}px</span>
        </div>

        <div className="ml-auto flex items-center gap-2">
          <span className="text-sm text-gray-500">
            {connectedUsers} 人在线
          </span>
          <button
            onClick={() => {
              setCanvasState(prev => ({ ...prev, elements: [] }))
              if (wsRef.current?.readyState === WebSocket.OPEN) {
                wsRef.current.send(JSON.stringify({
                  type: 'canvas_update',
                  payload: { elements: [] }
                }))
              }
            }}
            className="px-3 py-1 text-sm bg-red-500 text-white rounded hover:bg-red-600"
          >
            清空
          </button>
        </div>
      </div>
      
      <div ref={containerRef} className="flex-1 overflow-hidden">
        <canvas
          ref={canvasRef}
          onMouseDown={handleMouseDown}
          onMouseMove={handleMouseMove}
          onMouseUp={handleMouseUp}
          onMouseLeave={() => setIsDrawing(false)}
          className="cursor-crosshair"
        />
      </div>
    </div>
  )
}

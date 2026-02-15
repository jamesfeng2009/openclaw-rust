import { create } from 'zustand'
import { persist } from 'zustand/middleware'

export interface Channel {
  id: string
  type: string
  name: string
  enabled: boolean
}

export interface Agent {
  id: string
  name: string
  status: 'online' | 'idle' | 'offline'
}

export interface Message {
  id: string
  content: string
  sender: 'user' | 'agent' | 'system'
  timestamp: Date
}

export interface Session {
  id: string
  name: string
  channelId?: string
  agentId?: string
}

interface AppState {
  // UI State
  sidebarOpen: boolean
  setSidebarOpen: (open: boolean) => void
  
  // Theme
  darkMode: boolean
  setDarkMode: (dark: boolean) => void
  
  // Channels
  channels: Channel[]
  setChannels: (channels: Channel[]) => void
  toggleChannel: (id: string) => void
  
  // Agents
  agents: Agent[]
  setAgents: (agents: Agent[]) => void
  updateAgentStatus: (id: string, status: Agent['status']) => void
  
  // Messages
  messages: Message[]
  addMessage: (message: Message) => void
  clearMessages: () => void
  
  // Sessions
  sessions: Session[]
  currentSessionId: string | null
  setSessions: (sessions: Session[]) => void
  setCurrentSession: (id: string | null) => void
  
  // Gateway
  gatewayUrl: string
  setGatewayUrl: (url: string) => void
}

export const useAppStore = create<AppState>()(
  persist(
    (set) => ({
      // UI State
      sidebarOpen: true,
      setSidebarOpen: (open) => set({ sidebarOpen: open }),
      
      // Theme
      darkMode: false,
      setDarkMode: (dark) => set({ darkMode: dark }),
      
      // Channels
      channels: [],
      setChannels: (channels) => set({ channels }),
      toggleChannel: (id) => set((state) => ({
        channels: state.channels.map(c => 
          c.id === id ? { ...c, enabled: !c.enabled } : c
        )
      })),
      
      // Agents
      agents: [],
      setAgents: (agents) => set({ agents }),
      updateAgentStatus: (id, status) => set((state) => ({
        agents: state.agents.map(a =>
          a.id === id ? { ...a, status } : a
        )
      })),
      
      // Messages
      messages: [],
      addMessage: (message) => set((state) => ({
        messages: [...state.messages, message]
      })),
      clearMessages: () => set({ messages: [] }),
      
      // Sessions
      sessions: [],
      currentSessionId: null,
      setSessions: (sessions) => set({ sessions }),
      setCurrentSession: (id) => set({ currentSessionId: id }),
      
      // Gateway
      gatewayUrl: 'http://localhost:18789',
      setGatewayUrl: (url) => set({ gatewayUrl: url }),
    }),
    {
      name: 'openclaw-storage',
      partialize: (state) => ({
        darkMode: state.darkMode,
        gatewayUrl: state.gatewayUrl,
      }),
    }
  )
)

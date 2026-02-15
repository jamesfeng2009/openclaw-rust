export interface ApiResponse<T = unknown> {
  success: boolean
  data?: T
  error?: string
}

export interface ChannelInfo {
  id: string
  type: string
  name: string
  enabled: boolean
  config?: Record<string, unknown>
}

export interface AgentInfo {
  id: string
  name: string
  status: 'online' | 'idle' | 'offline'
  capabilities?: string[]
}

export interface SessionInfo {
  id: string
  name: string
  agentId?: string
  channelId?: string
  state: 'active' | 'idle' | 'closed'
  createdAt: string
  messageCount?: number
}

class ApiClient {
  private baseUrl: string
  
  constructor() {
    this.baseUrl = 'http://localhost:18789'
  }
  
  setBaseUrl(url: string) {
    this.baseUrl = url
  }
  
  private async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<ApiResponse<T>> {
    try {
      const response = await fetch(`${this.baseUrl}${endpoint}`, {
        ...options,
        headers: {
          'Content-Type': 'application/json',
          ...options.headers,
        },
      })
      
      const data = await response.json()
      
      if (!response.ok) {
        return {
          success: false,
          error: data.error || `HTTP ${response.status}`,
        }
      }
      
      return {
        success: true,
        data,
      }
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      }
    }
  }
  
  // Channels
  async getChannels(): Promise<ApiResponse<ChannelInfo[]>> {
    return this.request<ChannelInfo[]>('/api/channels')
  }
  
  async createChannel(config: Partial<ChannelInfo>): Promise<ApiResponse<ChannelInfo>> {
    return this.request<ChannelInfo>('/api/channels', {
      method: 'POST',
      body: JSON.stringify(config),
    })
  }
  
  async deleteChannel(id: string): Promise<ApiResponse<void>> {
    return this.request<void>(`/api/channels/${id}`, {
      method: 'DELETE',
    })
  }
  
  // Agents
  async getAgents(): Promise<ApiResponse<AgentInfo[]>> {
    return this.request<AgentInfo[]>('/api/agents')
  }
  
  async getAgent(id: string): Promise<ApiResponse<AgentInfo>> {
    return this.request<AgentInfo>(`/api/agents/${id}`)
  }
  
  async createAgent(config: Partial<AgentInfo>): Promise<ApiResponse<AgentInfo>> {
    return this.request<AgentInfo>('/api/agents', {
      method: 'POST',
      body: JSON.stringify(config),
    })
  }
  
  // Sessions
  async getSessions(): Promise<ApiResponse<SessionInfo[]>> {
    return this.request<SessionInfo[]>('/api/sessions')
  }
  
  async getSession(id: string): Promise<ApiResponse<SessionInfo>> {
    return this.request<SessionInfo>(`/api/sessions/${id}`)
  }
  
  async createSession(config: Partial<SessionInfo>): Promise<ApiResponse<SessionInfo>> {
    return this.request<SessionInfo>('/api/sessions', {
      method: 'POST',
      body: JSON.stringify(config),
    })
  }
  
  async closeSession(id: string): Promise<ApiResponse<void>> {
    return this.request<void>(`/api/sessions/${id}/close`, {
      method: 'POST',
    })
  }
  
  // Messages
  async sendMessage(message: string, sessionId?: string): Promise<ApiResponse<{ message: string }>> {
    return this.request<{ message: string }>('/api/agent/message', {
      method: 'POST',
      body: JSON.stringify({ message, sessionId }),
    })
  }
  
  // Presence
  async getPresence(): Promise<ApiResponse<Record<string, string>>> {
    return this.request<Record<string, string>>('/api/presence')
  }
  
  async setPresence(status: string): Promise<ApiResponse<void>> {
    return this.request<void>('/api/presence', {
      method: 'POST',
      body: JSON.stringify({ status }),
    })
  }
}

export const apiClient = new ApiClient()

export function useApi() {
  return {
    setBaseUrl: (url: string) => apiClient.setBaseUrl(url),
    channels: {
      list: () => apiClient.getChannels(),
      create: (config: Partial<ChannelInfo>) => apiClient.createChannel(config),
      delete: (id: string) => apiClient.deleteChannel(id),
    },
    agents: {
      list: () => apiClient.getAgents(),
      get: (id: string) => apiClient.getAgent(id),
      create: (config: Partial<AgentInfo>) => apiClient.createAgent(config),
    },
    sessions: {
      list: () => apiClient.getSessions(),
      get: (id: string) => apiClient.getSession(id),
      create: (config: Partial<SessionInfo>) => apiClient.createSession(config),
      close: (id: string) => apiClient.closeSession(id),
    },
    messages: {
      send: (message: string, sessionId?: string) => apiClient.sendMessage(message, sessionId),
    },
    presence: {
      get: () => apiClient.getPresence(),
      set: (status: string) => apiClient.setPresence(status),
    },
  }
}

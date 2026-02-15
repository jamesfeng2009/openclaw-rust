import { useState } from 'react'

export interface Skill {
  id: string
  name: string
  description: string
  version: string
  author?: string
  category: string
  tags: string[]
  enabled: boolean
  source: 'bundled' | 'managed' | 'workspace' | 'clawhub'
}

export function SkillsPage() {
  const [activeTab, setActiveTab] = useState<'bundled' | 'managed' | 'workspace' | 'clawhub'>('bundled')
  const [skills, setSkills] = useState<Skill[]>([
    { id: 'builtin.file_ops', name: '文件操作', description: '读取、写入、复制、移动文件和目录', version: '1.0.0', author: 'OpenClaw', category: 'Productivity', tags: ['文件', 'IO'], enabled: true, source: 'bundled' },
    { id: 'builtin.web_search', name: '网页搜索', description: '使用搜索引擎查找信息', version: '1.0.0', author: 'OpenClaw', category: 'Analysis', tags: ['搜索', '网络'], enabled: true, source: 'bundled' },
    { id: 'builtin.image_gen', name: '图像生成', description: '使用 AI 生成图像', version: '1.0.0', author: 'OpenClaw', category: 'Media', tags: ['图像', 'AI', '生成'], enabled: true, source: 'bundled' },
    { id: 'builtin.code_analyze', name: '代码分析', description: '分析代码结构、检测问题、优化建议', version: '1.0.0', author: 'OpenClaw', category: 'Development', tags: ['代码', '分析', '开发'], enabled: true, source: 'bundled' },
    { id: 'builtin.data_process', name: '数据处理', description: '处理和分析结构化数据', version: '1.0.0', author: 'OpenClaw', category: 'Analysis', tags: ['数据', '处理'], enabled: true, source: 'bundled' },
    { id: 'builtin.automation', name: '自动化任务', description: '创建和执行自动化工作流', version: '1.0.0', author: 'OpenClaw', category: 'Automation', tags: ['自动化', '工作流'], enabled: true, source: 'bundled' },
    { id: 'builtin.safe_execute', name: '安全执行', description: '在沙箱环境中安全执行代码', version: '1.0.0', author: 'OpenClaw', category: 'Security', tags: ['安全', '沙箱'], enabled: true, source: 'bundled' },
  ])

  const [clawhubSkills] = useState<Skill[]>([
    { id: 'clawhub.web_scraper', name: '网页抓取', description: '高效抓取网页内容', version: '1.2.0', author: 'Community', category: 'Utility', tags: ['爬虫', '网页'], enabled: false, source: 'clawhub' },
    { id: 'clawhub.pdf_tool', name: 'PDF 工具', description: 'PDF 创建、编辑和转换', version: '2.0.1', author: 'Community', category: 'Utility', tags: ['PDF', '文档'], enabled: false, source: 'clawhub' },
    { id: 'clawhub.ocr', name: 'OCR 文字识别', description: '从图像中提取文字', version: '1.5.0', author: 'Community', category: 'Utility', tags: ['OCR', '文字识别'], enabled: false, source: 'clawhub' },
  ])

  const toggleSkill = (skillId: string) => {
    setSkills(prev => prev.map(skill => 
      skill.id === skillId ? { ...skill, enabled: !skill.enabled } : skill
    ))
  }

  const installSkill = (skill: Skill) => {
    setSkills(prev => [...prev, { ...skill, enabled: true, source: 'managed' as const }])
  }

  const getCategoryColor = (category: string) => {
    const colors: Record<string, string> = {
      Productivity: 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400',
      Automation: 'bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-400',
      Analysis: 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400',
      Communication: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400',
      Development: 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400',
      Media: 'bg-pink-100 text-pink-800 dark:bg-pink-900/30 dark:text-pink-400',
      Security: 'bg-gray-100 text-gray-800 dark:bg-gray-900/30 dark:text-gray-400',
      Utility: 'bg-orange-100 text-orange-800 dark:bg-orange-900/30 dark:text-orange-400',
    }
    return colors[category] || 'bg-gray-100 text-gray-800'
  }

  const getSourceIcon = (source: Skill['source']) => {
    switch (source) {
      case 'bundled': return '✓'
      case 'managed': return '📦'
      case 'workspace': return '📁'
      case 'clawhub': return '🧩'
    }
  }

  const filteredSkills = activeTab === 'clawhub' ? clawhubSkills : skills.filter(s => s.source === activeTab)

  const tabs = [
    { id: 'bundled', label: '内置技能', count: skills.filter(s => s.source === 'bundled').length },
    { id: 'managed', label: '托管技能', count: skills.filter(s => s.source === 'managed').length },
    { id: 'workspace', label: '工作区', count: skills.filter(s => s.source === 'workspace').length },
    { id: 'clawhub', label: 'ClawHub', count: clawhubSkills.length },
  ] as const

  return (
    <div className="flex flex-col h-full">
      <div className="border-b border-gray-200 dark:border-gray-700">
        <nav className="flex space-x-8 px-6" aria-label="Tabs">
          {tabs.map(tab => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`py-4 px-1 border-b-2 font-medium text-sm ${
                activeTab === tab.id
                  ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                  : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
              }`}
            >
              {tab.label}
              <span className={`ml-2 rounded-full px-2 py-0.5 text-xs ${
                activeTab === tab.id 
                  ? 'bg-blue-100 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400'
                  : 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400'
              }`}>
                {tab.count}
              </span>
            </button>
          ))}
        </nav>
      </div>

      <div className="flex-1 p-6 overflow-y-auto">
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {filteredSkills.map(skill => (
            <div
              key={skill.id}
              className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-4"
            >
              <div className="flex items-start justify-between mb-2">
                <div className="flex items-center gap-2">
                  <span className="text-lg">{getSourceIcon(skill.source)}</span>
                  <h3 className="font-semibold">{skill.name}</h3>
                </div>
                <span className="text-xs text-gray-500">v{skill.version}</span>
              </div>
              
              <p className="text-sm text-gray-500 dark:text-gray-400 mb-3">{skill.description}</p>
              
              <div className="flex flex-wrap gap-2 mb-3">
                <span className={`px-2 py-1 rounded-full text-xs ${getCategoryColor(skill.category)}`}>
                  {skill.category}
                </span>
                {skill.tags.map(tag => (
                  <span key={tag} className="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded-full text-xs">
                    {tag}
                  </span>
                ))}
              </div>
              
              <div className="flex items-center justify-between">
                <span className="text-xs text-gray-500">
                  {skill.author && `作者: ${skill.author}`}
                </span>
                
                {activeTab === 'clawhub' ? (
                  <button
                    onClick={() => installSkill(skill)}
                    className="px-3 py-1 text-sm bg-blue-500 text-white rounded hover:bg-blue-600"
                  >
                    安装
                  </button>
                ) : (
                  <label className="relative inline-flex items-center cursor-pointer">
                    <input
                      type="checkbox"
                      checked={skill.enabled}
                      onChange={() => toggleSkill(skill.id)}
                      className="sr-only peer"
                    />
                    <div className="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                  </label>
                )}
              </div>
            </div>
          ))}
        </div>

        {filteredSkills.length === 0 && (
          <div className="flex items-center justify-center h-64 text-gray-500">
            {activeTab === 'workspace' ? '工作区技能为空' : 
             activeTab === 'managed' ? '暂无托管技能' : 
             '暂无技能'}
          </div>
        )}
      </div>
    </div>
  )
}

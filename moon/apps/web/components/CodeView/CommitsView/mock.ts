// 假设这是您的接口定义，为了在下面的数据中使用，我将它重复一次
export interface CommitMockItem {
  /** commit message 第一行 */
  message: string
  /** Verified，例如 Verified */
  Verified: string
  /** 作者 */
  assignees: string[]
  /** 短 id，例如 ec10365 */
  shortId: string
  /** 时间戳，用于排序（可选） */
  timestamp: number
}

// 获取当前时间戳
const NOW = Date.now()

// 辅助函数，用于计算不同日期的偏移时间戳
const dayInMs = 24 * 60 * 60 * 1000

/**
 * 先用一组静态数据占位，后续可以替换为真实接口返回
 */
export const mockCommits: CommitMockItem[] = [
  {
    message: 'feat: use libvault_core in vault',
    Verified: 'Verified',
    assignees: ['juanlou1217'],
    shortId: '8315c0a',
    timestamp: NOW - dayInMs - 2 * 60 * 60 * 1000 // 昨天 2小时前
  },
  {
    message: 'chore: update dependency versions',
    Verified: 'Unverified', // 模拟未验证提交
    assignees: ['juanlou1217', 'juanlou1217'],
    shortId: 'c7d8e9g',
    timestamp: NOW - dayInMs - 18 * 60 * 60 * 1000 // 昨天 18小时前
  },

  // --- 前几天 (Dec 30, 2025) ---
  {
    message: 'feat: third-party hides the New File/Folder Button',
    Verified: 'Verified',
    assignees: ['juanlou1217', 'juanlou1217'],
    shortId: '2938190',
    timestamp: NOW - 2 * dayInMs - 16 * 60 * 60 * 1000 // 前天 16小时前
  },
  {
    message: 'fix: address issue with mobile layout',
    Verified: 'Verified',
    assignees: ['juanlou1217', 'juanlou1217', 'juanlou1217', 'juanlou1217'], // 模拟 4 个作者 (>3)
    shortId: 'f0a1b2c',
    timestamp: NOW - 2 * dayInMs - 16 * 60 * 60 * 1000 // 前天 16小时前
  }
].sort((a, b) => b.timestamp - a.timestamp) // 确保数据按时间倒序排列

export const mockMembers = [
  {
    user: {
      username: 'juanlou1217',
      display_name: 'Juan Lou',
      avatar_url: ''
    }
  }
]

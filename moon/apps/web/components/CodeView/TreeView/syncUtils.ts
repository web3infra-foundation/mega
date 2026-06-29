const GITHUB_SYNC_ROOTS = ['third-party', 'project'] as const

export function canShowGitHubSync(path: string[], version: string): boolean {
  return version === 'main' && GITHUB_SYNC_ROOTS.includes(path[0] as (typeof GITHUB_SYNC_ROOTS)[number])
}

export function isProjectSyncPath(currentPath?: string): boolean {
  if (!currentPath) return false
  return currentPath === '/project' || currentPath.startsWith('/project/')
}

export interface ParsedGitHubRepo {
  owner: string
  repo: string
}

/** Parse github.com URLs or `owner/repo` shorthand into owner and repo name. */
export function parseGitHubRepoUrl(input: string): ParsedGitHubRepo | null {
  const trimmed = input.trim()

  if (!trimmed) return null

  const shorthand = trimmed.match(/^([a-zA-Z0-9_.-]+)\/([a-zA-Z0-9_.-]+)$/)

  if (shorthand) {
    return {
      owner: shorthand[1],
      repo: shorthand[2].replace(/\.git$/i, '')
    }
  }

  let url: URL

  try {
    const withProtocol = trimmed.includes('://') ? trimmed : `https://${trimmed}`

    url = new URL(withProtocol)
  } catch {
    return null
  }

  const host = url.hostname.toLowerCase()

  if (host !== 'github.com' && host !== 'www.github.com') {
    return null
  }

  const segments = url.pathname.split('/').filter(Boolean)

  if (segments.length < 2) return null

  const owner = segments[0]
  const repo = segments[1].replace(/\.git$/i, '')

  if (!owner || !repo) return null

  return { owner, repo }
}

export function formatGitHubRepoUrl(owner: string, repo: string): string {
  return `https://github.com/${owner}/${repo}`
}

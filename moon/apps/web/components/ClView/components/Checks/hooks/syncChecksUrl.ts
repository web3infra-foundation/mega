/** Update tab=check and build query params without triggering a Next.js route transition. */
export function syncChecksUrl(buildId: string) {
  if (typeof window === 'undefined' || !buildId) return

  const url = new URL(window.location.href)

  if (url.searchParams.get('tab') === 'check' && url.searchParams.get('build') === buildId) {
    return
  }

  url.searchParams.set('tab', 'check')
  url.searchParams.set('build', buildId)

  const href = `${url.pathname}${url.search}${url.hash}`

  window.history.replaceState(window.history.state, '', href)
}

export function readBuildFromUrl(routerBuild: string | string[] | undefined): string | undefined {
  if (typeof routerBuild === 'string' && routerBuild) return routerBuild

  if (typeof window !== 'undefined') {
    return new URLSearchParams(window.location.search).get('build') ?? undefined
  }

  return undefined
}

export function readTabFromUrl(routerTab: string | string[] | undefined): string | undefined {
  if (typeof routerTab === 'string' && routerTab) return routerTab

  if (typeof window !== 'undefined') {
    return new URLSearchParams(window.location.search).get('tab') ?? undefined
  }

  return undefined
}

import { COMMUNITY_SLUG } from '@gitmono/config'

import { useScope } from '@/contexts/scope'

export function useIsCommunity() {
  const { scope } = useScope()

  return scope === COMMUNITY_SLUG
}

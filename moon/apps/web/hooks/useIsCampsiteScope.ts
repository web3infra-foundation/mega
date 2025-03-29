import { CAMPSITE_SCOPE } from '@gitmono/config'

import { useScope } from '@/contexts/scope'

export function useIsCampsiteScope() {
  const { scope } = useScope()

  return { isCampsiteScope: scope === CAMPSITE_SCOPE }
}

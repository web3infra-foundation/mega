import { AnimatePresence, m } from 'framer-motion'
import { useAtomValue } from 'jotai'

import { ANIMATION_CONSTANTS, CONTAINER_STYLES, UIText } from '@gitmono/ui'
import { cn } from '@gitmono/ui/src/utils'

import { callToastsAtom } from '@/atoms/callToasts'

interface Props {
  className?: string
}

export function CallToasts({ className }: Props) {
  const callToasts = useAtomValue(callToastsAtom)

  if (callToasts.size === 0) return null

  return (
    <div className={cn('mx-2 flex flex-col items-center gap-3', className)}>
      {Array.from(callToasts)
        .reverse()
        .map(([id, message]) => (
          <AnimatePresence key={id}>
            <m.div
              className={cn(
                CONTAINER_STYLES.base,
                CONTAINER_STYLES.shadows,
                ANIMATION_CONSTANTS,
                'bg-elevated dark rounded-lg px-3 py-2 text-center'
              )}
            >
              <UIText secondary>{message}</UIText>
            </m.div>
          </AnimatePresence>
        ))}
    </div>
  )
}

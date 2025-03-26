import { useAutoplayError } from '@100mslive/react-sdk'

import { Button } from '@gitmono/ui/Button'
import { UIText } from '@gitmono/ui/Text'
import { ANIMATION_CONSTANTS, cn, CONTAINER_STYLES } from '@gitmono/ui/utils'

export function AutoplayBlockedError() {
  const { error, resetError, unblockAudio } = useAutoplayError()

  if (!error) return null

  return (
    <div
      className={cn(
        CONTAINER_STYLES.base,
        CONTAINER_STYLES.shadows,
        ANIMATION_CONSTANTS,
        'bg-elevated dark absolute flex flex-col gap-3 rounded-lg p-4 text-center'
      )}
    >
      <UIText>Your browser has blocked audio.</UIText>
      <Button
        variant='primary'
        onClick={() => {
          unblockAudio()
          resetError()
        }}
      >
        Allow Audio
      </Button>
    </div>
  )
}

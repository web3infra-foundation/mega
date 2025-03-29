import { isMobile } from 'react-device-detect'

import { UIText } from '@gitmono/ui'
import { cn } from '@gitmono/ui/src/utils'

export function DeactivatedMemberThreadComposer({ plural }: { plural?: boolean }) {
  const text = plural
    ? 'These people are no longer active in your organization'
    : 'This person is no longer active in your organization'

  return (
    <div
      className={cn('bg-primary border-t p-4 text-center', {
        'pb-safe-offset-2 [&:has(.ProseMirror-focused)]:pb-2': isMobile
      })}
    >
      <UIText quaternary>{text}</UIText>
    </div>
  )
}

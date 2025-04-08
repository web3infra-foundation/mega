import { TimelineEvent } from '@gitmono/types/generated'
import { RelativeTime } from '@gitmono/ui/RelativeTime'
import { UIText } from '@gitmono/ui/Text'
import { Tooltip } from '@gitmono/ui/Tooltip'

import { longTimestamp } from '@/utils/timestamp'

interface TimelineEventCreatedAtTextProps {
  timelineEvent: TimelineEvent
}

export function TimelineEventCreatedAtText({ timelineEvent }: TimelineEventCreatedAtTextProps) {
  const createdAtTitle = longTimestamp(timelineEvent.created_at)

  return (
    <Tooltip label={createdAtTitle}>
      <UIText element='span' quaternary className='ml-1.5' size='text-inherit'>
        <RelativeTime time={timelineEvent.created_at} />
      </UIText>
    </Tooltip>
  )
}

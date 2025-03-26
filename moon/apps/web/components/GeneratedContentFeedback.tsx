import { useState } from 'react'
import toast from 'react-hot-toast'

import { Button } from '@gitmono/ui/Button'
import { ThumbsDownIcon, ThumbsUpIcon } from '@gitmono/ui/Icons'
import { UIText } from '@gitmono/ui/Text'
import { Tooltip } from '@gitmono/ui/Tooltip'
import { cn } from '@gitmono/ui/utils'

interface Props {
  responseId: string
  feature: string
  className?: string
}

export function GeneratedContentFeedback(props: Props) {
  return <InnerGeneratedContentFeedback key={props.responseId} {...props} />
}

function InnerGeneratedContentFeedback({ className }: Props) {
  const [response, setResponse] = useState<null | 'positive' | 'negative'>(null)

  function handleClick(value: 'positive' | 'negative') {
    if (response === value) {
      return
    }
    setResponse(value)
    toast('Thanks for your feedback!')
  }

  return (
    <div className={cn('text-secondary flex items-center', className)}>
      <Tooltip label='This content was generated with AI'>
        <UIText inherit size='text-xs' className='mr-2'>
          Was this helpful?
        </UIText>
      </Tooltip>
      <Button
        size='sm'
        iconOnly={<ThumbsUpIcon size={16} />}
        onClick={() => handleClick('positive')}
        tooltip='Helpful'
        accessibilityLabel='Helpful'
        variant={response === 'positive' ? 'flat' : 'plain'}
        disabled={!!response}
      />
      <Button
        size='sm'
        iconOnly={<ThumbsDownIcon size={16} />}
        onClick={() => handleClick('negative')}
        tooltip='Not helpful'
        accessibilityLabel='Not helpful'
        variant={response === 'negative' ? 'flat' : 'plain'}
        disabled={!!response}
      />
    </div>
  )
}

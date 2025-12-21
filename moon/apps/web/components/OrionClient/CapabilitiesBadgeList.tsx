'use client'

import { Badge, UIText } from '@gitmono/ui'

export function CapabilitiesBadgeList({ capabilities }: { capabilities?: string | string[] }) {
  if (!capabilities || (Array.isArray(capabilities) && capabilities.length === 0)) {
    return <UIText color='text-muted'>—</UIText>
  }

  const list = Array.isArray(capabilities)
    ? capabilities
    : capabilities
        .split(',')
        .map((c) => c.trim())
        .filter(Boolean)

  return (
    <div className='flex flex-wrap gap-1'>
      {list.map((cap) => (
        <Badge key={cap} color='default' className='capitalize'>
          {cap}
        </Badge>
      ))}
    </div>
  )
}

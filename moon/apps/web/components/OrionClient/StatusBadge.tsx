'use client'

import { Badge } from '@gitmono/ui'

import { OrionClientStatus } from './types'

type UIOrionClientStatus = Exclude<OrionClientStatus, 'preparing' | 'uploading'>

const STATUS_META: Record<
  UIOrionClientStatus,
  { color: 'default' | 'blue' | 'green' | 'brand' | 'orange' | 'amber'; label: string }
> = {
  idle: { color: 'green', label: 'Idle' },
  busy: { color: 'orange', label: 'Busy ' },
  running: { color: 'blue', label: 'Running build' },
  downloading: { color: 'blue', label: 'Downloading source' },
  error: { color: 'brand', label: 'Error ' },
  offline: { color: 'default', label: 'Lost / Offline' }
}

export function StatusBadge({ status }: { status: OrionClientStatus }) {
  const normalizedStatus: UIOrionClientStatus = status
  const meta = STATUS_META[normalizedStatus]

  return (
    <Badge className='min-w-[140px] justify-center px-3 py-0.5 text-xs' color={meta.color}>
      {meta.label}
    </Badge>
  )
}

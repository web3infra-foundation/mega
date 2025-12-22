'use client'

import { Badge } from '@gitmono/ui'

import { OrionClientStatus } from './types'

const STATUS_META: Record<
  OrionClientStatus,
  { color: 'default' | 'blue' | 'green' | 'brand' | 'orange' | 'amber'; label: string }
> = {
  idle: { color: 'green', label: 'Idle' },
  busy: { color: 'orange', label: 'Busy ' },
  running: { color: 'blue', label: 'Running build' },
  downloading: { color: 'blue', label: 'Downloading source' },
  preparing: { color: 'blue', label: 'Preparing environment' },
  uploading: { color: 'blue', label: 'Uploading artifacts' },
  error: { color: 'brand', label: 'Error ' },
  offline: { color: 'default', label: 'Lost / Offline' }
}

export function StatusBadge({ status }: { status: OrionClientStatus }) {
  const meta = STATUS_META[status] ?? STATUS_META.idle

  return (
    <Badge className='min-w-[140px] justify-center px-3 py-0.5 text-xs' color={meta.color}>
      {meta.label}
    </Badge>
  )
}

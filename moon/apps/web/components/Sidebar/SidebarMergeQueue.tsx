import { GitMergeQueueIcon } from '@gitmono/ui/Icons'

import { SidebarLink, SidebarProps } from './SidebarLink'

export function SidebarMergeQueue({ label = 'Merge Queue', href, active }: SidebarProps) {
  return <SidebarLink id='mq' label={label} href={href} active={active} leadingAccessory={<GitMergeQueueIcon />} />
}

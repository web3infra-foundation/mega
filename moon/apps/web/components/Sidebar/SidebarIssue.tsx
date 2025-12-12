import { ChatBubblePlusIcon } from '@gitmono/ui/Icons'

import { SidebarLink, SidebarProps } from './SidebarLink'

export function SidebarIssue({ label = 'Issue', href, active }: SidebarProps) {
  return <SidebarLink id='issue' label={label} href={href} active={active} leadingAccessory={<ChatBubblePlusIcon />} />
}

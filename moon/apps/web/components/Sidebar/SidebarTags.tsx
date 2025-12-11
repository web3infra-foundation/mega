import { TagIcon } from '@gitmono/ui/Icons'

import { SidebarLink, SidebarProps } from './SidebarLink'

export function SidebarTags({ label = 'Tags', href, active }: SidebarProps) {
  return <SidebarLink id='tags' label={label} href={href} active={active} leadingAccessory={<TagIcon />} />
}

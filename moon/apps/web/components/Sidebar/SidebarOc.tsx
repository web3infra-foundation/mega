import { ServerStackIcon } from '@gitmono/ui/Icons'

import { SidebarLink, SidebarProps } from './SidebarLink'

export function SidebarOc({ label = 'Orion Client', href, active }: SidebarProps) {
  return (
    <SidebarLink id='oc' label={label} href={href} active={active} leadingAccessory={<ServerStackIcon size={20} />} />
  )
}

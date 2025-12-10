import { ComponentIcon } from '@gitmono/ui/Icons'

import { SidebarLink, SidebarProps } from './SidebarLink'

export function SidebarCode({ label = 'Code', href, active }: SidebarProps) {
  return (
    <>
      <SidebarLink id='code' label={label} href={href} active={active} leadingAccessory={<ComponentIcon />} />
    </>
  )
}

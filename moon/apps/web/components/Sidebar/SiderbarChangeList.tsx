import { GitCommitIcon } from '@gitmono/ui/Icons'

import { SidebarLink, SidebarProps } from './SidebarLink'

export function SiderbarChangeList({ label = 'Change List', href, active }: SidebarProps) {
  return (
    <>
      <SidebarLink id='cl' label={label} href={href} active={active} leadingAccessory={<GitCommitIcon />} />
    </>
  )
}

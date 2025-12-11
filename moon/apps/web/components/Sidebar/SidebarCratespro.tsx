import Image from 'next/image'

import { SidebarLink, SidebarProps } from './SidebarLink'

export function SidebarCratespro({ label = 'Rust', href, active }: SidebarProps) {
  return (
    <SidebarLink
      id='rust'
      label={label}
      href={href}
      active={active}
      leadingAccessory={<Image src='/rust/Rust-Tour-Doc.png' alt='Rust Logo' width={21} height={21} />}
    />
  )
}

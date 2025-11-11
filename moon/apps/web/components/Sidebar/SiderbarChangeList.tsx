import router from 'next/router'

import { GitCommitIcon } from '@gitmono/ui/Icons'

import { useScope } from '@/contexts/scope'

import { SidebarLink } from './SidebarLink'

export function SiderbarChangeList() {
  const { scope } = useScope()

  return (
    <>
      <SidebarLink
        id='cl'
        label='Change List'
        href={`/${scope}/cl`}
        active={router.pathname === '/[org]/cl'}
        leadingAccessory={<GitCommitIcon />}
      />
    </>
  )
}

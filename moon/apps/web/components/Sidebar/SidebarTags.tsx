import router from 'next/router'

import { TagIcon } from '@gitmono/ui/Icons'

import { useScope } from '@/contexts/scope'

import { SidebarLink } from './SidebarLink'

export function SidebarTags() {
  const { scope } = useScope()

  return (
    <>
      <SidebarLink
        id='tags'
        label='Tags'
        href={`/${scope}/code/tags`}
        active={router.pathname === '/[org]/code/tags'}
        leadingAccessory={<TagIcon />}
      />
    </>
  )
}

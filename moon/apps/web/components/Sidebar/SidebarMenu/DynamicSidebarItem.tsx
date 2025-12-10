import React, { useMemo } from 'react'
import { useRouter } from 'next/router'

import { Vec } from '@gitmono/types/generated'

import { SidebarCode } from '@/components/Sidebar/SidebarCode'
import { SidebarCratespro } from '@/components/Sidebar/SidebarCratespro'
import { SidebarDrafts } from '@/components/Sidebar/SidebarDrafts'
import { SidebarInbox } from '@/components/Sidebar/SidebarInbox'
import { SidebarIssue } from '@/components/Sidebar/SidebarIssue'
import { SidebarLink } from '@/components/Sidebar/SidebarLink'
import { SidebarMergeQueue } from '@/components/Sidebar/SidebarMergeQueue'
import { SidebarCalls, SidebarDocs, SidebarHome, SidebarMessages } from '@/components/Sidebar/SidebarMyWorkGroup'
import { SidebarTags } from '@/components/Sidebar/SidebarTags'
import { SiderbarChangeList } from '@/components/Sidebar/SiderbarChangeList'
import { useScope } from '@/contexts/scope'

export interface DynamicSidebarItemProps {
  config: Vec[number]
}

export function DynamicSidebarItem({ config }: DynamicSidebarItemProps) {
  const router = useRouter()
  const { scope } = useScope()

  const componentMap = useMemo(
    () => ({
      home: SidebarHome,
      inbox: SidebarInbox,
      chat: SidebarMessages,
      notes: SidebarDocs,
      calls: SidebarCalls,
      drafts: SidebarDrafts,
      code: SidebarCode,
      tags: SidebarTags,
      cl: SiderbarChangeList,
      mq: SidebarMergeQueue,
      issue: SidebarIssue,
      rust: SidebarCratespro
    }),
    []
  )

  if (!config.visible) {
    return null
  }

  const href = `/${scope}${config.href}`
  const isActive = router.asPath === href || router.asPath.startsWith(`${href}/`)

  const Component = componentMap[config.public_id as keyof typeof componentMap]

  if (!Component) {
    return <SidebarLink id={config.public_id} label={config.label} href={href} active={isActive} />
  }

  return <Component label={config.label} href={href} active={isActive} />
}

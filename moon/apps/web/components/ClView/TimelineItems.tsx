import React, { useMemo } from 'react'

import '@primer/primitives/dist/css/functional/themes/light.css'

import {
  CheckCircleIcon,
  CommentIcon,
  FeedMergedIcon,
  FeedPullRequestClosedIcon,
  FeedPullRequestOpenIcon,
  FeedTagIcon,
  GitPullRequestDraftIcon,
  PersonIcon,
  RepoPushIcon
} from '@primer/octicons-react'
import { BaseStyles, ThemeProvider, Timeline } from '@primer/react'

import { ConversationItem, ReviewerInfo } from '@gitmono/types/generated'

import CLComment from '@/components/ClView/CLComment'
import LabelItem from '@/components/ClView/LabelItem'
import ReviewComment from '@/components/ClView/ReviewComment'
import { CommonDetailData } from '@/utils/types'

import { SimpleNoteContentRef } from '../SimpleNoteEditor/SimpleNoteContent'
import AssigneeItem from './AssigneeItem'
import CloseItem from './CloseItem'
import EditItem from './EditItem'
import ForcePushItem from './item/ForcePushItem'
import MergedItem from './MergedItem'
import ReopenItem from './ReopenItem'

interface TimelineItemProps {
  badge?: React.ReactNode
  children?: React.ReactNode
  isOver?: boolean
}

interface ConvItem {
  id: number
  badge?: React.ReactNode
  children?: React.ReactNode
  isOver: boolean
}

interface TimelineWrapperProps {
  convItems?: ConvItem[]
}

const TimelineItem = React.memo(({ badge, children, isOver }: TimelineItemProps) => {
  return (
    <>
      <Timeline.Item>
        <Timeline.Badge>{badge}</Timeline.Badge>
        <Timeline.Body style={{ marginTop: '4px' } as React.CSSProperties}>{children}</Timeline.Body>
      </Timeline.Item>
      {isOver && <Timeline.Break />}
    </>
  )
})

TimelineItem.displayName = 'TimelineItem'

const TimelineWrapper = React.memo<TimelineWrapperProps>(({ convItems = [] }) => {
  return (
    <ThemeProvider>
      <BaseStyles>
        <Timeline clipSidebar>
          {convItems?.map((item) => (
            <TimelineItem key={item.id} badge={item.badge} isOver={item.isOver}>
              {item.children}
            </TimelineItem>
          ))}
        </Timeline>
      </BaseStyles>
    </ThemeProvider>
  )
})

TimelineWrapper.displayName = 'TimelineWrapper'

const TimelineItems = React.memo<{
  detail: CommonDetailData
  id: string
  type: string
  editorRef: React.RefObject<SimpleNoteContentRef>
  reviewers?: ReviewerInfo[]
}>(({ detail, id, type, editorRef, reviewers = [] }) => {
  const convItems: ConvItem[] = useMemo(
    () =>
      detail!!.conversations.map((conv: ConversationItem) => {
        let icon
        let children
        let isOver = false

        const isCurrentReviewer = reviewers.some((r) => r.username === conv.username)

        switch (conv.conv_type) {
          case 'Comment':
            icon = <CommentIcon />
            children = <CLComment conv={conv} id={id} whoamI={type} editorRef={editorRef} />
            break
          case 'Review':
            if (!isCurrentReviewer) {
              icon = <CommentIcon />
              children = <CLComment conv={conv} id={id} whoamI={type} editorRef={editorRef} />
            } else {
              icon = <CheckCircleIcon size={24} className='text-blue-500' />
              children = <ReviewComment reviewers={reviewers} conv={conv} id={id} whoamI={type} editorRef={editorRef} />
            }
            break
          case 'Merged':
            icon = <FeedMergedIcon size={24} className='text-purple-500' />
            children = <MergedItem conv={conv} />
            isOver = true
            break
          case 'Closed':
            icon = <FeedPullRequestClosedIcon size={24} className='text-red-600' />
            children = <CloseItem conv={conv} />
            isOver = true
            break
          case 'Edit':
            icon = <GitPullRequestDraftIcon size={24} className='text-[#6e7781]' />
            children = <EditItem conv={conv} />
            break
          case 'Reopen':
            icon = <FeedPullRequestOpenIcon size={24} className='text-green-500' />
            children = <ReopenItem conv={conv} />
            break
          case 'Assignee':
            icon = <PersonIcon size={24} />
            children = <AssigneeItem conv={conv} />
            break
          case 'Label':
            icon = <FeedTagIcon size={24} className='text-cyan-500' />
            children = <LabelItem conv={conv} />
            break
          case 'ForcePush':
            icon = <RepoPushIcon />
            children = <ForcePushItem conv={conv} />
            break
        }

        return { badge: icon, children, isOver, id: conv.id }
      }),
    [detail, id, type, editorRef, reviewers]
  )

  return <TimelineWrapper convItems={convItems} />
})

TimelineItems.displayName = 'TimelineItems'

export default TimelineItems

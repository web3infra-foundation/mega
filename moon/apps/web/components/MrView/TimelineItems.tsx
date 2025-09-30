import React from 'react'

import '@primer/primitives/dist/css/functional/themes/light.css'

import {
  CommentIcon,
  FeedMergedIcon,
  FeedPullRequestClosedIcon,
  FeedPullRequestOpenIcon,
  FeedTagIcon,
  PersonIcon,
  RepoPushIcon,
  CheckCircleIcon
} from '@primer/octicons-react'
import { BaseStyles, ThemeProvider, Timeline } from '@primer/react'

import { ConversationItem } from '@gitmono/types/generated'
import { CommonDetailData } from '@/utils/types'

import MRComment from '@/components/MrView/MRComment'
import ReviewComment from '@/components/MrView/ReviewComment'
import AssigneeItem from './AssigneeItem'
import CloseItem from './CloseItem'
import MergedItem from './MergedItem'
import ReopenItem from './ReopenItem'
import LabelItem from '@/components/MrView/LabelItem'
import ForcePushItem from './item/ForcePushItem'
import { SimpleNoteContentRef } from '../SimpleNoteEditor/SimpleNoteContent'

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

const TimelineItem = ({ badge, children, isOver }: TimelineItemProps) => {
  return (
    <>
      <Timeline.Item>
        <Timeline.Badge>{badge}</Timeline.Badge>
        <Timeline.Body>{children}</Timeline.Body>
      </Timeline.Item>
      {isOver && <Timeline.Break />}
    </>
  )
}

const TimelineWrapper: React.FC<TimelineWrapperProps> = ({ convItems = [] }) => {
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
}

const TimelineItems: React.FC<{detail: CommonDetailData; id: string; type: string; editorRef: React.RefObject<SimpleNoteContentRef> }> = ({ detail, id, type, editorRef }) => {
  const assignees = detail!!.assignees!!
  const convItems: ConvItem[] = detail!!.conversations.map((conv: ConversationItem) => {
    let icon
    let children
    let isOver = false

    switch (conv.conv_type) {
      case 'Comment':
        icon = <CommentIcon />
        children = <MRComment conv={conv} id={id} whoamI={type} editorRef={editorRef} />
        break
      case 'Review':
        icon = <CheckCircleIcon size={24} className='text-blue-500' />
        children = <ReviewComment assignees={assignees} conv={conv} id={id} whoamI={type} editorRef={editorRef} />
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
  }) !!

  return <TimelineWrapper convItems={convItems} />
}

export default TimelineItems

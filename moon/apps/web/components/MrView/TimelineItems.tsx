import React from 'react'
import '@primer/primitives/dist/css/functional/themes/light.css'
import { BaseStyles, ThemeProvider, Timeline } from '@primer/react'
import { FeedPullRequestClosedIcon, CommentIcon, FeedMergedIcon, FeedPullRequestOpenIcon } from '@primer/octicons-react'
import MRComment from '@/components/MrView/MRComment';
import CloseItem from './CloseItem';
import ReopenItem from './ReopenItem';
import MergedItem from './MergedItem';
import { ConversationItem } from '@gitmono/types/generated'

interface TimelineItemProps {
  badge?: React.ReactNode
  children?: React.ReactNode
  isOver?: boolean
}

interface ConvItem {
  id: number
  badge?: React.ReactNode
  children?: React.ReactNode
  isOver: boolean;
}

interface TimelineWrapperProps {
  convItems?: ConvItem[];
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
  );
};

const TimelineItems: React.FC<{ detail: any, id: string, type: string }> = ({ detail, id, type }) => {

  const convItems: ConvItem[] = detail.conversations.map((conv: ConversationItem) => {
    let icon;
    let children;
    let isOver = false;

    switch (conv.conv_type) {
      case 'Comment':
        icon = <CommentIcon />;
        children = <MRComment conv={conv} id={id} whoamI={type} />;
        break;
      case 'Merged':
        icon = <FeedMergedIcon size={24} className="text-purple-500" />;
        children = <MergedItem conv={conv} />;
        isOver = true;
        break;
      case 'Closed':
        icon = <FeedPullRequestClosedIcon size={24} className="text-red-600" />;
        children = <CloseItem conv={conv}/>;
        isOver = true;
        break;
      case 'Reopen':
        icon = <FeedPullRequestOpenIcon size={24} 
        className = "text-green-500"/>
        children = <ReopenItem conv={conv}/>;
        break;
    }

    return { badge: icon, children, isOver, id: conv.id };
  });

  return <TimelineWrapper convItems={convItems} />;
};

export default TimelineItems;
import React from 'react'
import '@primer/primitives/dist/css/functional/themes/light.css'
import { BaseStyles, ThemeProvider, Timeline } from '@primer/react'
import { FeedPullRequestClosedIcon, CommentIcon, FeedMergedIcon, IssueReopenedIcon } from '@primer/octicons-react'
import MRComment from '@/components/MrView/MRComment';
import { formatDistance, fromUnixTime } from 'date-fns';
import { MRDetail } from '@/pages/[org]/mr/[id]';

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

const TimelineItems: React.FC<{ mrDetail?: MRDetail, id: string }> = ({ mrDetail, id }) => {
  if (!mrDetail) {
    return null; 
  }

  const convItems: ConvItem[] = mrDetail?.conversations.map((conv) => {
    let icon;
    let children;
    let isOver = false;

    switch (conv.conv_type) {
      case 'Comment':
        icon = <CommentIcon />;
        children = <MRComment conv={conv} id={id} whoamI="mr" />;
        break;
      case 'Merged':
        icon = <FeedMergedIcon size={24} className="text-purple-500" />;
        children =
          'Merged via the queue into main ' +
          formatDistance(fromUnixTime(conv.created_at), new Date(), { addSuffix: true });
        isOver = true;
        break;
      case 'Closed':
        icon = <FeedPullRequestClosedIcon size={24} className="text-red-500" />;
        children = conv.comment;
        isOver = true;
        break;
      case 'Reopen':
        icon = <IssueReopenedIcon />;
        children = conv.comment;
        break;
    }

    return { badge: icon, children, isOver, id: conv.id };
  });

  return <TimelineWrapper convItems={convItems} />;
};

export default TimelineItems;
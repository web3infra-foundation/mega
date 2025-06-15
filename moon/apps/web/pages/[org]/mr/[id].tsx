'use client'

import React, { useState } from 'react';
import { Card, Tabs, TabsProps,Timeline,} from 'antd';
// import { CommentOutlined, MergeOutlined, CloseCircleOutlined, PullRequestOutlined } from '@ant-design/icons';
import { ChevronRightCircleIcon, ChevronSelectIcon,AlarmIcon,ClockIcon} from '@gitmono/ui/Icons'
import { formatDistance, fromUnixTime } from 'date-fns';
import RichEditor from '@/components/MrView/rich-editor/RichEditor';
import MRComment from '@/components/MrView/MRComment';
import { useRouter } from 'next/router';
import 'diff2html/bundles/css/diff2html.min.css';
import FileDiff from '@/components/DiffView/FileDiff';
import { Button } from '@gitmono/ui';
// import { ReloadIcon } from '@radix-ui/react-icons';
import {DownloadIcon } from '@gitmono/ui'
import { cn } from '@gitmono/ui/utils';
import AuthAppProviders from '@/components/Providers/AuthAppProviders';
import { AppLayout } from '@/components/Layout/AppLayout';
import { PageWithLayout } from '@/utils/types';
import { useGetMrDetail } from '@/hooks/useGetMrDetail'
import { useGetMrFilesChanged } from '@/hooks/useGetMrFilesChanged';
import { usePostMrComment } from '@/hooks/usePostMrComment'
import { usePostMrMerge } from '@/hooks/usePostMrMerge'
import { usePostMrReopen } from '@/hooks/usePostMrReopen';
import { usePostMrClose } from '@/hooks/usePostMrClose';
import { useScope } from '@/contexts/scope'

interface MRDetail {
    status: string,
    conversations: Conversation[],
    title: string,
}
export interface Conversation {
    id: number,
    user_id: number,
    conv_type: string,
    comment: string,
    created_at: number,
}

const  MRDetailPage:PageWithLayout<any> = () =>{
    const router = useRouter();
    const { id : tempId, title } = router.query;
    const { scope } = useScope()
    const [editorState, setEditorState] = useState("");
    const [editorHasText, setEditorHasText] = useState(false);
    const [login, _setLogin] = useState(true);

    const id = typeof tempId === 'string' ? tempId : '';
    const { data: MrDetailData } = useGetMrDetail(id)
    const mrDetail = MrDetailData?.data as MRDetail | undefined

    
    if (mrDetail && typeof mrDetail.status === 'string') {
      mrDetail.status = mrDetail.status.toLowerCase();
    }

    const { data: MrFilesChangedData} = useGetMrFilesChanged(id)

    const { mutate: approveMr, isPending : mrMergeIsPending } = usePostMrMerge(id)
    const handleMrApprove = () => {
      approveMr(undefined, {
        onSuccess: () => {
          router.push(`/${scope}/mr`)
        },
      })
    }

    const { mutate: closeMr, isPending: mrCloseIsPending } = usePostMrClose(id)
    const handleMrClose = () => {
      closeMr(undefined, {
        onSuccess: () => {
          router.push(`/${scope}/mr`)
        },
      })
    }

    const { mutate: reopenMr, isPending: mrReopenIsPending } = usePostMrReopen(id)
    const handleMrReopen = () => {
      reopenMr(undefined,{
        onSuccess: () => {
          router.push(`/${scope}/mr`)
        },
      })
    }

    const { mutate: postMrComment, isPending : mrCommentIsPending } = usePostMrComment(id)
    const save_comment = () => {
      postMrComment(
        { content: editorState },
        {
        onSuccess: () => {
          setEditorState("");
        },
      });
    }

    let conv_items = mrDetail?.conversations.map(conv => {
        let icon;
        let children;

        switch (conv.conv_type) {
            case "Comment": icon = <ChevronRightCircleIcon />; children = <MRComment conv={conv} id={id} whoamI='mr'/>; break
            case "Merged": icon = <ChevronSelectIcon />; children = "Merged via the queue into main " + formatDistance(fromUnixTime(conv.created_at), new Date(), { addSuffix: true }); break;
            case "Closed": icon = <AlarmIcon />; children = conv.comment; break;
            case "Reopen": icon = <ClockIcon />; children = conv.comment; break;
        };

        const element = {
            dot: icon,
            // color: 'red',
            children: children
        }

        return element
    });

    const buttonClasses= 'cursor-pointer';

    const tab_items: TabsProps['items'] = [
      {
        key: '1',
        label: 'Conversation',
        children:
          <div className="flex flex-col w-full">
            <Timeline items={conv_items}/>
            <h1>Add a comment</h1>
            <RichEditor setEditorState={setEditorState} setEditorHasText={setEditorHasText}/>
            <div className="flex gap-2 justify-end">
              {mrDetail && mrDetail.status === "open" &&
                <Button
                  disabled={!login || mrCloseIsPending}
                  onClick={handleMrClose}
                  aria-label="Close Merge Request"
                  className={cn(buttonClasses)}
                >
                  {mrCloseIsPending && <DownloadIcon className="mr-2 h-4 w-4 animate-spin"/>}
                  Close Merge Request
                </Button>
              }
              {mrDetail && mrDetail.status === "closed" &&
                <Button
                  disabled={!login || mrReopenIsPending}
                  onClick={handleMrReopen}
                  aria-label="Reopen Merge Request"
                  className={cn(buttonClasses)}
                >
                  {mrReopenIsPending && <DownloadIcon className="mr-2 h-4 w-4 animate-spin"/>}
                  Reopen Merge Request
                </Button>
              }
              <Button
                disabled={!login || !editorHasText}
                onClick={() => save_comment()}
                aria-label="Comment"
                className={cn(buttonClasses)}
              >
                {mrCommentIsPending && <DownloadIcon className="mr-2 h-4 w-4 animate-spin"/>}
                Comment
              </Button>
            </div>
          </div>
      },
      {
        key: '2',
        label: 'Files Changed',
        children: MrFilesChangedData?.data?.content ? 
                  <FileDiff diffs={MrFilesChangedData.data.content} /> : 
                  <div>No files changed</div>
      }
    ];

    return (
      <Card title={title + " #" + id} className="h-screen overflow-auto">
          {mrDetail && mrDetail.status === "open" &&
            <Button
              disabled={!login || mrMergeIsPending}
              onClick={handleMrApprove}
              aria-label="Merge MR"
              className={cn(buttonClasses)}
            >
                {mrMergeIsPending && <DownloadIcon className="mr-2 h-4 w-4 animate-spin"/>}
                Merge MR
            </Button>
          }
          <Tabs defaultActiveKey="1" items={tab_items} />
      </Card>
    )
}


MRDetailPage.getProviders = (page, pageProps) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default MRDetailPage

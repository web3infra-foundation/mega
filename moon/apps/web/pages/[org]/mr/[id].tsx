'use client'

import React, { useCallback, useEffect, useState } from 'react';
import { Card, Tabs, TabsProps,Timeline,} from 'antd';
// import { CommentOutlined, MergeOutlined, CloseCircleOutlined, PullRequestOutlined } from '@ant-design/icons';
import { ChevronRightCircleIcon, ChevronSelectIcon,AlarmIcon,ClockIcon} from '@gitmono/ui/Icons'
import { formatDistance, fromUnixTime } from 'date-fns';
import RichEditor from '@/components/MrView/rich-editor/RichEditor';
import MRComment from '@/components/MrView/MRComment';
import { useParams, useRouter } from 'next/navigation';
import * as Diff2Html from 'diff2html';
import 'diff2html/bundles/css/diff2html.min.css';
import FilesChanged from '@/components/MrView/files-changed';
import { Button } from '@gitmono/ui';
// import { ReloadIcon } from '@radix-ui/react-icons';
import {DownloadIcon } from '@gitmono/ui'
import { cn } from '@gitmono/ui/utils';
import AuthAppProviders from '@/components/Providers/AuthAppProviders';
import { AppLayout } from '@/components/Layout/AppLayout';
import { PageWithLayout } from '@/utils/types';

interface MRDetail {
    status: string,
    conversations: Conversation[],
    title: string,
}
interface Conversation {
    id: number,
    user_id: number,
    conv_type: string,
    comment: string,
    created_at: number,
}

const  MRDetailPage:PageWithLayout<any> = () =>{
    // const { id } = React.use(params)
    const router = useRouter();
    const params = useParams();

    const id = params?.id as string;

    const [editorState, setEditorState] = useState("");
    const [login, setLogin] = useState(false);
    const [mrDetail, setMrDetail] = useState<MRDetail>(
        {
            status: "",
            conversations: [],
            title: "",
        }
    );
    const [filedata, setFileData] = useState([]);
    const [loadings, setLoadings] = useState<boolean[]>([]);
    const [outputHtml, setOutputHtml] = useState('');

    const checkLogin = async () => {
        const res = await fetch(`/api/auth`);

        setLogin(res.ok);
    };

    const fetchDetail = useCallback(async () => {
        const detail = await fetch(`/api/mr/${id}/detail`);
        const detail_json = await detail.json();

        setMrDetail(detail_json.data.data);
    }, [id]);

    const fetchFileList = useCallback(async () => {
        set_to_loading(2)
        try {
            const res = await fetch(`/api/mr/${id}/files`);
            const result = await res.json();

            setFileData(result.data.data);
        } finally {
            cancel_loading(2)
        }
    }, [id]);

    const get_diff_content = useCallback(async () => {
        const detail = await fetch(`/api/mr/${id}/files-changed`);
        const res = await detail.json();
        const diff = Diff2Html.html(res.data.data.content, { drawFileList: true, matching: 'lines' })

        setOutputHtml(diff);
    }, [id])

    useEffect(() => {
        fetchDetail()
        fetchFileList();
        checkLogin();
        
    }, [id, fetchDetail, fetchFileList]);
    
    useEffect(()=>{
      // eslint-disable-next-line no-console
      console.log(filedata);
    },[filedata])

    const set_to_loading = (index: number) => {
        setLoadings((prevLoadings) => {
            const newLoadings = [...prevLoadings];

            newLoadings[index] = true;
            return newLoadings;
        });
    }

    const cancel_loading = (index: number) => {
        setLoadings((prevLoadings) => {
            const newLoadings = [...prevLoadings];

            newLoadings[index] = false;
            return newLoadings;
        });
    }

    async function approve_mr() {
        set_to_loading(1);
        const res = await fetch(`/api/mr/${id}/merge`, {
            method: 'POST',
        });

        if (res) {
            cancel_loading(1);
            router.push(
                "/mr"
            );
        }
    };

    async function close_mr() {
        set_to_loading(3);
        const res = await fetch(`/api/mr/${id}/close`, {
            method: 'POST',
        });

        if (res) {
            cancel_loading(3);
            router.push(
                "/mr"
            );
        }
    };

    async function reopen_mr() {
        set_to_loading(3);
        const res = await fetch(`/api/mr/${id}/reopen`, {
            method: 'POST',
        });

        if (res) {
            cancel_loading(3);
            router.push(
                "/mr"
            );
        }
    };


    async function save_comment(comment:any) {
        set_to_loading(3);
        const res = await fetch(`/api/mr/${id}/comment`, {
            method: 'POST',
            body: comment,
        });

        if (res) {
            setEditorState("");
            fetchDetail();
            cancel_loading(3);
        }
    }

    let conv_items = mrDetail?.conversations.map(conv => {
        let icon;
        let children;

        switch (conv.conv_type) {
            case "Comment": icon = <ChevronRightCircleIcon />; children = <MRComment conv={conv} fetchDetail={fetchDetail} />; break
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

    const onTabsChange = (key: string) => {
        // eslint-disable-next-line no-console
        console.log(key);
        if (key === '2') {
            get_diff_content()
        }
    };

    const buttonClasses= 'cursor-pointer';

    const tab_items: TabsProps['items'] = [
      {
        key: '1',
        label: 'Conversation',
        children:
          <div className="flex flex-col w-full">
            <Timeline items={conv_items}/>
            <h1>Add a comment</h1>
            <RichEditor setEditorState={setEditorState}/>
            <div className="flex gap-2 justify-end">
              {mrDetail && mrDetail.status === "open" &&
                <Button
                  disabled={!login}
                  onClick={() => close_mr()}
                  aria-label="Close Merge Request"
                  className={cn(buttonClasses)}
                >
                  {loadings[3] && <DownloadIcon className="mr-2 h-4 w-4 animate-spin"/>}
                  Close Merge Request
                </Button>
              }
              {mrDetail && mrDetail.status === "closed" &&
                <Button
                  disabled={!login}
                  onClick={() => reopen_mr()}
                  aria-label="Reopen Merge Request"
                  className={cn(buttonClasses)}
                >
                  {loadings[3] && <DownloadIcon className="mr-2 h-4 w-4 animate-spin"/>}
                  Reopen Merge Request
                </Button>
              }
              <Button
                disabled={!login}
                onClick={() => save_comment(editorState)}
                aria-label="Comment"
                className={cn(buttonClasses)}
              >
                {loadings[3] && <DownloadIcon className="mr-2 h-4 w-4 animate-spin"/>}
                Comment
              </Button>
            </div>
          </div>
      },
      {
        key: '2',
        label: 'Files Changed',
        children: <FilesChanged outputHtml={outputHtml}/>
      }
    ];

    return (
      <Card title={mrDetail.title + " #" + id}>
          {mrDetail && mrDetail.status === "open" &&
            <Button
              disabled={!login}
              onClick={() => approve_mr()}
              aria-label="Merge MR"
              className={cn(buttonClasses)}
            >
                {loadings[1] && <DownloadIcon className="mr-2 h-4 w-4 animate-spin"/>}
                Merge MR
            </Button>
          }
          <Tabs defaultActiveKey="1" items={tab_items} onChange={onTabsChange}/>
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

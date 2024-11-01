'use client'
import { useEffect, useState } from "react";
import { Card, Button, List, Tabs, TabsProps, Space, Timeline, Flex } from 'antd/lib';
import { CommentOutlined, MergeOutlined, CloseCircleOutlined, PullRequestOutlined } from '@ant-design/icons';
import { formatDistance, fromUnixTime } from 'date-fns';
import RichEditor from "@/components/rich-editor/RichEditor";
import MRComment from "@/components/MRComment";
import * as React from 'react'
import { useRouter } from "next/navigation";

interface MRDetail {
    status: string,
    conversions: Conversation[],
    title: string,
}
interface Conversation {
    id: number,
    user_id: number,
    conv_type: string,
    comment: string,
    created_at: number,
}

type Params = Promise<{ id: string }>

export default function MRDetailPage({ params }: { params: Params }) {
    const { id } = React.use(params)

    const [editorState, setEditorState] = useState("");
    const [login, setLogin] = useState(false);
    const [mrDetail, setMrDetail] = useState<MRDetail>(
        {
            status: "",
            conversions: [],
            title: "",
        }
    );
    const [filedata, setFileData] = useState([]);
    const [loadings, setLoadings] = useState<boolean[]>([]);
    const router = useRouter();

    const checkLogin = async () => {
        const res = await fetch(`/api/auth`);
        setLogin(res.ok);
    };

    const fetchDetail = async () => {
        const detail = await fetch(`/api/mr/${id}/detail`);
        const detail_json = await detail.json();
        setMrDetail(detail_json.data.data);
    };

    const fetchFileList = async () => {
        set_to_loading(2)
        try {
            const res = await fetch(`/api/mr/${id}/files`);
            const result = await res.json();
            setFileData(result.data.data);
        } finally {
            cancel_loading(2)
        }
    };

    useEffect(() => {
        fetchDetail()
        fetchFileList();
        checkLogin();
    }, [id]);

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


    async function save_comment(comment) {
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

    let conv_items = mrDetail?.conversions.map(conv => {
        let icon;
        let children;
        switch (conv.conv_type) {
            case "Comment": icon = <CommentOutlined />; children = <MRComment conv={conv} fetchDetail={fetchDetail} />; break
            case "Merged": icon = <MergeOutlined />; children = "Merged via the queue into main " + formatDistance(fromUnixTime(conv.created_at), new Date(), { addSuffix: true }); break;
            case "Closed": icon = <CloseCircleOutlined />; children = conv.comment; break;
            case "Reopen": icon = <PullRequestOutlined />; children = conv.comment; break;
        };

        const element = {
            dot: icon,
            // color: 'red',
            children: children
        }
        return element
    });

    const tab_items: TabsProps['items'] = [
        {
            key: '1',
            label: 'Conversation',
            children:
                <Space direction="vertical" style={{ width: '100%' }}>
                    <Timeline items={conv_items} />
                    <h1>Add a comment</h1>
                    <RichEditor setEditorState={setEditorState} />
                    <Flex gap="small" justify={"flex-end"}>
                        {mrDetail && mrDetail.status === "open" &&
                            <Button loading={loadings[3]} disabled={!login} onClick={() => close_mr()}>Close Merge Request</Button>
                        }
                        {mrDetail && mrDetail.status === "closed" &&
                            <Button loading={loadings[3]} disabled={!login} onClick={() => reopen_mr()}>Reopen Merge Request</Button>
                        }
                        <Button loading={loadings[3]} disabled={!login} onClick={() => save_comment(editorState)}>Comment</Button>
                    </Flex>
                </Space>
        },
        {
            key: '2',
            label: 'Files Changed',
            children: <Space style={{ width: '100%' }}>
                <List
                    header={<div>Change File List</div>}
                    bordered
                    dataSource={filedata}
                    loading={loadings[2]}
                    renderItem={(item) => (
                        <List.Item>
                            {item}
                        </List.Item>
                    )}
                />
            </Space>
        }
    ];

    return (
        <Card title={mrDetail.title + " #" + id}>
            {mrDetail && mrDetail.status === "open" &&
                <Button
                    loading={loadings[1]}
                    onClick={() => approve_mr()}
                    disabled={!login}
                >
                    Merge MR
                </Button>
            }
            <Tabs defaultActiveKey="1" items={tab_items} />
        </Card>
    )
}

'use client'

import React, { useCallback, useEffect, useState } from 'react';
import { Card, Button, Tabs, TabsProps, Space, Timeline, Flex } from 'antd';
import { CommentOutlined, CloseCircleOutlined } from '@ant-design/icons';
import RichEditor from '@/components/rich-editor/RichEditor';
import MRComment from '@/components/MRComment';
import { useRouter } from 'next/navigation';

interface IssueDetail {
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

type Params = Promise<{ id: string }>

export default function IssueDetailPage({ params }: { params: Params }) {
    const { id } = React.use(params)

    const [editorState, setEditorState] = useState("");
    const [login, setLogin] = useState(false);
    const [info, setInfo] = useState<IssueDetail>(
        {
            status: "",
            conversations: [],
            title: "",
        }
    );
    const [loadings, setLoadings] = useState<boolean[]>([]);
    const router = useRouter();

    const fetchDetail = useCallback(async () => {
        const detail = await fetch(`/api/issue/${id}/detail`);
        const detail_json = await detail.json();
        setInfo(detail_json.data.data);
    }, [id]);

    const checkLogin = async () => {
        const res = await fetch(`/api/auth`);
        setLogin(res.ok);
    };

    useEffect(() => {
        checkLogin()
        fetchDetail()
    }, [id, fetchDetail]);

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

    async function close_issue() {
        set_to_loading(3);
        const res = await fetch(`/api/issue/${id}/close`, {
            method: 'POST',
        });
        if (res) {
            router.push(
                "/issue"
            );
        }
    };

    async function reopen_issue() {
        set_to_loading(3);
        const res = await fetch(`/api/issue/${id}/reopen`, {
            method: 'POST',
        });
        if (res) {
            router.push(
                "/issue"
            );
        }
    };

    async function save_comment(comment) {
        set_to_loading(3);
        const res = await fetch(`/api/issue/${id}/comment`, {
            method: 'POST',
            body: comment,
        });
        if (res) {
            setEditorState("");
            fetchDetail();
            cancel_loading(3);
        }
    }

    const conv_items = info?.conversations.map(conv => {
        let icon;
        let children;
        switch (conv.conv_type) {
            case "Comment": icon = <CommentOutlined />; children = <MRComment conv={conv} fetchDetail={fetchDetail} />; break
            case "Closed": icon = <CloseCircleOutlined />; children = conv.comment;
        };

        const element = {
            dot: icon,
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
                    {info && info.status === "open" &&
                        <>
                            <h1>Add a comment</h1>
                            <RichEditor setEditorState={setEditorState} />
                            <Flex gap="small" justify={"flex-end"}>
                                <Button loading={loadings[3]} disabled={!login} onClick={() => close_issue()}>Close issue</Button>
                                <Button loading={loadings[3]} disabled={editorState === "" || !login} onClick={() => save_comment(editorState)}>Comment</Button>
                            </Flex>
                        </>
                    }
                    {info && info.status === "closed" &&
                        <Flex gap="small" justify={"flex-end"}>
                            <Button loading={loadings[3]} disabled={!login} onClick={() => reopen_issue()}>Reopen issue</Button>
                        </Flex>
                    }
                </Space>
        }
    ];

    return (
        <Card title={info.title + " #" + id}>
            <Tabs defaultActiveKey="1" items={tab_items} />
        </Card>
    )
}

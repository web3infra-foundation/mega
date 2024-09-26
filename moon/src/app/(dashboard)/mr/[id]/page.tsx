'use client'
import { useEffect, useState } from "react";
import { Card, Button, List, Tabs, TabsProps, Space, Timeline, Flex } from 'antd/lib';
import { CommentOutlined, MergeOutlined } from '@ant-design/icons';
import { formatDistance, fromUnixTime } from 'date-fns';
import RichEditor from "@/components/rich-editor/RichEditor";
import MRComment from "@/components/MRComment";

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

export default function MRDetailPage({ params }: { params: { id: string } }) {
    const [editorState, setEditorState] = useState("");
    const [mrDetail, setMrDetail] = useState<MRDetail>(
        {
            status: "",
            conversions: [],
            title: "",
        }
    );
    const [filedata, setFileData] = useState([]);
    const [loadings, setLoadings] = useState<boolean[]>([]);

    const fetchDetail = async () => {
        const detail = await fetch(`/api/mr/${params.id}/detail`);
        const detail_json = await detail.json();
        setMrDetail(detail_json.data.data);
    };

    const fetchFileList = async () => {
        set_to_loading(2)
        try {
            const res = await fetch(`/api/mr/${params.id}/files`);
            const result = await res.json();
            setFileData(result.data.data);
        } finally {
            cancel_loading(2)
        }
    };

    useEffect(() => {
        fetchDetail()
        fetchFileList();
    }, [params.id]);

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
        const res = await fetch(`/api/mr/${params.id}/merge`, {
            method: 'POST',
        });
        if (res) {
            cancel_loading(1);
        }
    };

    async function save_comment(comment) {
        set_to_loading(3);
        const res = await fetch(`/api/mr/${params.id}/comment`, {
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
            // default: icon = <CommentOutlined />; children = conv.comment;
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
                    <Flex justify={"flex-end"}>
                        <Button loading={loadings[3]} onClick={() => save_comment(editorState)}>Comment</Button>
                    </Flex>
                </Space>
        },
        {
            key: '2',
            label: 'Files Changed',
            children: <Space style={{ width: '100%' }}>
                <List
                    // style={{ width: '100%' }}
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
        <Card title={mrDetail.title + " #" + params.id}>
            {mrDetail && mrDetail.status === "open" &&
                <Button
                    loading={loadings[1]}
                    onClick={() => approve_mr()}
                >
                    Merge MR
                </Button>
            }
            <Tabs defaultActiveKey="1" items={tab_items} />
        </Card>
    )
}

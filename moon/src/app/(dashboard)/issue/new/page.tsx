'use client'
import { useState } from "react";
import { Button, Input, Space, Flex } from 'antd/lib';
import RichEditor from "@/components/rich-editor/RichEditor";
import { useRouter } from "next/navigation";

export default function MRDetailPage() {
    const [editorState, setEditorState] = useState("");
    const [title, setTitle] = useState("");
    const [loadings, setLoadings] = useState<boolean[]>([]);
    const router = useRouter();

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

    async function submit(description) {
        set_to_loading(3);
        const res = await fetch(`/api/issue/new`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                title: title,
                description: description
            }),
        });
        if (res) {
            setEditorState("");
            cancel_loading(3);
            router.push(
                "/issue"
            );
        }
    }


    return (
        <>
            <Space direction="vertical" style={{ width: '100%' }}>
                <h1>Add a title
                    <Input aria-label="title" name="title" placeholder="Title" value={title} onChange={(e) => setTitle(e.target.value)}></Input>
                </h1>
            </Space>
            <Space direction="vertical" style={{ width: '100%' }}>
                <h1>Add a description</h1>
                <RichEditor setEditorState={setEditorState} />
                <Flex justify={"flex-end"}>
                    <Button loading={loadings[3]} onClick={() => submit(editorState)}>Submit New Issue</Button>
                </Flex>
            </Space>
        </>
    )
}

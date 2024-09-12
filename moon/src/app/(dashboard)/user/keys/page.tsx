'use client'

import { Divider } from '@/components/catalyst/divider'
import { Heading } from '@/components/catalyst/heading'
import { useEffect, useState } from 'react'
import { Flex, Button, message, List } from "antd";
import {
    KeyIcon,
} from '@heroicons/react/20/solid'
import { format } from 'date-fns'

interface KeyItem {
    id: number,
    ssh_key: string,
    created_at: string,
    finger: string,
    title: string,
}

export default function KeysPage() {
    const [keyList, setKeyList] = useState([]);
    const [messageApi, contextHolder] = message.useMessage();

    const error = () => {
        messageApi.open({
            type: 'error',
            content: 'Delete failed!',
        });
    };
    const success = () => {
        messageApi.open({
            type: 'success',
            content: 'Delete success!',
        });
    };

    const fetchData = async () => {
        try {
            const res = await fetch(`/api/user/ssh`);
            const response = await res.json();
            const keyList = response.data.data;
            setKeyList(keyList);
        } catch (error) {
            console.error('Error fetching data:', error);
        }
    };

    useEffect(() => {
        fetchData();
    }, []);

    const delete_ssh_key = async (id) => {
        const res = await fetch(`/api/user/ssh/${id}/delete`, {
            method: 'POST',
        });
        if (res.ok) {
            success();
            fetchData();
        } else {
            error();
        }
    }

    return (
        <>
            {contextHolder}
            <Heading>SSH Keys</Heading>
            <Flex justify={'flex-end'} >
                <Button style={{ backgroundColor: '#428646' }} href='/user/keys/add'>New SSH Key</Button>
            </Flex>
            <Divider className="my-10 mt-6" />
            This is a list of SSH keys associated with your account. Remove any keys that you do not recognize.
            <br />
            <List
                size="large"
                bordered
                dataSource={keyList as KeyItem[]}
                renderItem={(item) =>
                    <List.Item>
                        <List.Item.Meta
                            avatar={
                                <KeyIcon className="size-6" />
                            }
                            title={
                                <>
                                    {item.title}
                                    <br />
                                    SHA256: {item.finger}
                                </>
                            }
                            description={`Added on ${format(new Date(item.created_at), "MMM dd,yyyy")}`}
                        />
                        <Button danger onClick={() => delete_ssh_key(item.id)} >Delete</Button>
                    </List.Item>}
            />
        </>

    )
}



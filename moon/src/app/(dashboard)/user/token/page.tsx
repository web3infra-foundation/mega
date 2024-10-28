'use client'

import { Divider } from '@/components/catalyst/divider'
import { Heading } from '@/components/catalyst/heading'
import { useEffect, useState } from 'react'
import { Flex, Button, message, List, Modal } from "antd";
import {
    KeyIcon,
} from '@heroicons/react/20/solid'
import { format } from 'date-fns'
import copy from 'copy-to-clipboard';


interface TokenItem {
    id: number,
    token: string,
    created_at: string,
}

export default function KeysPage() {
    const [keyList, setKeyList] = useState([]);
    const [messageApi, contextHolder] = message.useMessage();
    const [isModalOpen, setIsModalOpen] = useState(false);
    const [token, setToken] = useState("");

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

    const generateToken = async () => {
        try {
            const res = await fetch(`/api/user/token`, {
                method: 'POST',
            });
            const response = await res.json();
            const token = response.data.data;
            setToken(token);
        } catch (error) {
            console.error('Error fetching data:', error);
        }
    };

    const showModal = () => {
        setIsModalOpen(true);
        generateToken();
    };

    const handleOk = () => {
        setIsModalOpen(false);
        copy(token)
        fetchToken()
    };

    const handleCancel = () => {
        setIsModalOpen(false);
    };

    const fetchToken = async () => {
        try {
            const res = await fetch(`/api/user/token`);
            const response = await res.json();
            const keyList = response.data.data;
            setKeyList(keyList);
        } catch (error) {
            console.error('Error fetching data:', error);
        }
    };

    useEffect(() => {
        fetchToken();
    }, []);

    const delete_ssh_key = async (id) => {
        const res = await fetch(`/api/user/token/${id}/delete`, {
            method: 'POST',
        });
        if (res.ok) {
            success();
            fetchToken();
        } else {
            error();
        }
    }

    return (
        <>
            {contextHolder}
            <Heading>Access Tokens</Heading>
            <Flex justify={'flex-end'} >
                <Button style={{ backgroundColor: '#428646' }} onClick={showModal} >Generate Token</Button>
            </Flex>
            <Modal title="Generated Token, Only show once, Please Copy it before close" open={isModalOpen} onOk={handleOk} onCancel={handleCancel}>
                <p>{token}</p>
            </Modal>
            <Divider className="my-10 mt-6" />
            This is a list of access token associated with your account. Remove any keys that you do not recognize.
            <br />
            <List
                size="large"
                bordered
                dataSource={keyList as TokenItem[]}
                renderItem={(item) =>
                    <List.Item>
                        <List.Item.Meta
                            avatar={
                                <KeyIcon className="size-6" />
                            }
                            title={
                                <>
                                    {item.token}
                                </>
                            }
                            description={`Generated on ${format(new Date(item.created_at), "MMM dd,yyyy")}`}
                        />
                        <Button danger onClick={() => delete_ssh_key(item.id)} >Delete</Button>
                    </List.Item>}
            />
        </>

    )
}



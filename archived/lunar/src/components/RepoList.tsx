'use client'

import { Space, Table, TableProps, Badge, Button, Skeleton, message } from 'antd/lib'
import { useState } from 'react'
import { format, fromUnixTime } from 'date-fns'
import { DownloadOutlined } from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { ApiResult, useMegaStatus } from '@/app/api/fetcher';

const endpoint = process.env.NEXT_PUBLIC_API_URL;

interface DataType {
    name: string;
    identifier: string;
    origin: string;
    update_time: number;
    commit: string;
    peer_online: boolean
}

const DataList = ({ data }) => {
    const [messageApi, contextHolder] = message.useMessage();
    const { status, isLoading, isError } = useMegaStatus();
    const [loadings, setLoadings] = useState<boolean[]>([]);

    if (isLoading) return <Skeleton />;

    const enterLoading = (index: number) => {
        setLoadings((prevLoadings) => {
            const newLoadings = [...prevLoadings];
            newLoadings[index] = true;
            return newLoadings;
        });
    }

    const exitLoading = (index: number) => {
        setLoadings((prevLoadings) => {
            const newLoadings = [...prevLoadings];
            newLoadings[index] = false;
            return newLoadings;
        });
    }

    const msg_error = () => {
        messageApi.open({
            type: 'error',
            content: 'Failed to clone repo',
        });
    };

    const msg_success = () => {
        messageApi.open({
            type: 'success',
            content: 'Clone success',
        });
    };


    var columns: TableProps<DataType>['columns'] = [
        {
            title: 'Name',
            dataIndex: ['name', 'identifier'],
            key: 'name',
            render: (_, record) => {
                return <>
                    <Space>
                        <span>{record.name}</span>
                    </Space>
                </>
            }
        },
        {
            title: 'Identifier',
            dataIndex: 'identifier',
            key: 'identifier',
            ellipsis: true,
            render: (text) => <a>{text}</a>,
        },
        {
            title: 'Online',
            dataIndex: 'peer_online',
            key: 'peer_online',
            render: (_, { peer_online }) => (
                <Badge status={peer_online ? "success" : "default"} text={peer_online ? "On" : "Off"} />
            ),
        },
        {
            title: 'Origin',
            dataIndex: 'origin',
            key: 'origin',
        },
        {
            title: 'Update Date',
            dataIndex: 'update_time',
            key: 'update_time',
            render: (update_time) => (
                <Space>
                    <span>
                        {update_time && format(fromUnixTime(update_time), 'yyyy-MM-dd HH:mm:ss')}
                    </span>
                </Space>
            )
        },
        {
            title: 'Action',
            key: 'action',
            render: (_, record) => (
                <Space size="middle">
                    <Button disabled={!record.peer_online || !status[1]} loading={loadings[1]}
                        onClick={() => handleClone(record)} type="primary" shape="round" icon={<DownloadOutlined />} size={'small'}>
                        Clone
                    </Button>
                </Space>
            ),
        },
    ];

    const handleClone = async (record) => {
        enterLoading(1)
        try {
            let res: ApiResult<string> = await getRepoFork(record.identifier);
            // showSuccModel(text);
            console.log("repo fork result", res);
            if (res.req_result) {
                invoke('clone_repository', { repoUrl: res.data, name: record.name })
                    .then(() => msg_success())
                    .catch((error) => {
                        console.error(`Failed to get service status: ${error}`);
                        msg_error();
                    });
            } else {
                msg_error();
            }
        } catch (error) {
            console.error('Error fetching data:', error);
        }
        exitLoading(1)
    };

    return (
        <div>
            <Table scroll={{ x: true }} columns={columns} dataSource={data} />
            {/* <Modal
                title="Please input a local port"
                open={open}
                onOk={handleOk}
                confirmLoading={confirmLoading}
                onCancel={handleCancel}
                okButtonProps={{ disabled: isOkButtonDisabled }}
            >
                <Input showCount maxLength={5} value={inputPort} onChange={handleInputChange} />
            </Modal> */}
            {contextHolder}
        </div>
    );
};


async function getRepoFork(identifier) {
    const res = await fetch(`${endpoint}/api/v1/mega/ztm/repo_fork?identifier=${identifier}`);
    const response = await res.json();
    return response
}

export default DataList;

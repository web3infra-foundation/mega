'use client'

import 'github-markdown-css/github-markdown-light.css'
import { useRouter, useSearchParams } from 'next/navigation'
import Markdown from 'react-markdown'
import { formatDistance, fromUnixTime } from 'date-fns'
import styles from './CodeTable.module.css'
import { Input, Modal, Space, Table, TableProps, message } from 'antd/lib'
import { useState } from 'react'
import {
    FolderIcon,
    DocumentIcon,
} from '@heroicons/react/20/solid'
import { requestPublishRepo } from '@/app/api/fetcher'
import { Button } from '@/components/catalyst/button'

export interface DataType {
    oid: string;
    name: string;
    content_type: string;
    message: string;
    date: number;
}

const CodeTable = ({ directory, readmeContent, with_ztm }) => {
    const [messageApi, contextHolder] = message.useMessage();
    const msg_error = (content: String) => {
        messageApi.open({
            type: 'error',
            content: content,
        });
    };
    const msg_success = (content: String) => {
        messageApi.open({
            type: 'success',
            content: content,
        });
    };

    const router = useRouter();
    const fileCodeContainerStyle = {
        width: '100%',
        margin: '0 auto',
        borderRadius: '0.5rem',
        marginTop: '10px'
    };
    const [open, setOpen] = useState(false);
    const [confirmLoading, setConfirmLoading] = useState(false);
    const [modalText, setModalText] = useState('');
    const searchParams = useSearchParams();
    const path = searchParams.get('path');

    var columns: TableProps<DataType>['columns'] = [
        {
            title: 'Name',
            dataIndex: ['name', 'content_type'],
            key: 'name',
            render: (_, record) => {
                return <>
                    {record.content_type === "file" &&
                        <Space>
                            <DocumentIcon className="size-6" />
                            <span onClick={() => handleFileClick(record)}>{record.name}</span>
                        </Space>
                    }
                    {record.content_type === "directory" &&
                        <Space>
                            <FolderIcon className="size-6" />
                            <a onClick={() => handleDirectoryClick(record)}>{record.name}</a>
                        </Space>}
                </>
            }
        },
        {
            title: 'Message',
            dataIndex: 'message',
            key: 'message',
            render: (text) => <a>{text}</a>,
        },
        {
            title: 'Date',
            dataIndex: 'date',
            key: 'date',
            render: (_, { date }) => (
                <>
                    {date && formatDistance(fromUnixTime(date), new Date(), { addSuffix: true })}
                </>
            )
        },
        {
            title: 'Action',
            key: 'action',
            render: (_, record) => (
                <Space size="middle">
                    <Button disabled={!with_ztm} onClick={() => showModal(record.name)}>Publish</Button>
                    {/* <Button disabled={!with_ztm} outline>Revoke</Button> */}
                </Space>
            ),
        },
    ];

    const handleFileClick = (file) => {
        router.push(`/blob?path=${path}/${file.name}`);
    };

    const handleDirectoryClick = async (directory) => {
        var newPath = '';
        if (!path) {
            newPath = `/tree?path=/${directory.name}`;
        } else {
            newPath = `/tree?path=${path}/${directory.name}`;
        }
        router.push(
            newPath
        );
    };

    // const handleGoBack = () => {
    //     const safePath = real_path.split('/');
    //     if (safePath.length == 1) {
    //         router.push('/')
    //     } else {
    //         router.push(`/tree/${safePath.slice(0, -1).join('/')}`);
    //     }
    // };

    // sort by file type, render folder type first
    const sortedDir = directory.sort((a, b) => {
        if (a.content_type === 'directory' && b.content_type === 'file') {
            return -1;
        } else if (a.content_type === 'file' && b.content_type === 'directory') {
            return 1;
        } else {
            return 0;
        }
    });

    const showModal = (name) => {
        setModalText(name);
        setOpen(true);
    };

    const handleOk = async (filename) => {
        var newPath = '';
        if (!path) {
            newPath = `/${filename}`;
        } else {
            newPath = `${path}/${filename}`;
        }
        setConfirmLoading(true);

        try {
            const result = await requestPublishRepo({
                "path": newPath,
                "alias": filename,
            });
            msg_success("Publish Success!");
            console.log('Repo published successfully:', result);
        } catch (error) {
            msg_error("Publish failed:" + error);
        }
        setOpen(false);
        setConfirmLoading(false);
    };

    const handleCancel = () => {
        setOpen(false);
    };

    return (
        <div style={fileCodeContainerStyle}>
            {contextHolder}
            <Table style={{ clear: "none" }} rowClassName={styles.dirShowTr} pagination={false} columns={columns} dataSource={sortedDir} />
            <Modal
                title="Given a alias for repo to public"
                open={open}
                destroyOnClose
                onOk={() => handleOk(modalText)}
                confirmLoading={confirmLoading}
                onCancel={handleCancel}
            >
                <Input showCount maxLength={20} defaultValue={modalText} />
            </Modal>
            {readmeContent && (
                <div className={styles.markdownContent}>
                    <div className="markdown-body">
                        <Markdown>{readmeContent}</Markdown>
                    </div>
                </div>
            )}
        </div>
    );
};



export default CodeTable;

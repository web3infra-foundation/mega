
import 'github-markdown-css/github-markdown-light.css'
import { useRouter } from 'next/router'
import Markdown from 'react-markdown'
import { formatDistance, fromUnixTime } from 'date-fns'
import folderPic from '../../public/icons/folder.svg'
import filePic from '../../public/icons/file.svg'
import Image from 'next/image'
import styles from './CodeTable.module.css'
import { Input, Modal, Space, Table, TableProps } from 'antd/lib'
import { useState } from 'react'

export interface DataType {
    oid: string;
    name: string;
    content_type: string;
    message: string;
    date: number;
}

const CodeTable = ({ directory, readmeContent, showTree }) => {

    const router = useRouter();
    const fileCodeContainerStyle = showTree ? { width: '80%', marginLeft: '17%', borderRadius: '0.5rem', marginTop: '10px' } : { width: '90%', margin: '0 auto', borderRadius: '0.5rem', marginTop: '10px' };
    const [open, setOpen] = useState(false);
    const [confirmLoading, setConfirmLoading] = useState(false);
    const [modalText, setModalText] = useState('');

    var columns: TableProps<DataType>['columns'] = [
        {
            title: 'Name',
            dataIndex: ['name', 'content_type'],
            key: 'name',
            render: (_, record) => {
                return <>
                    {record.content_type === "file" &&
                        <Space>
                            <Image src={filePic} alt="File icon" className={styles.fileTableIcon} />
                            <span onClick={() => handleFileClick(record)}>{record.name}</span>
                        </Space>
                    }
                    {record.content_type === "directory" &&
                        <Space>
                            <Image src={folderPic} alt="File icon" className={styles.fileTableIcon} />
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
                    <a onClick={() => showModal(record.name)}>Publish</a>
                    <a>Revoke</a>
                </Space>
            ),
        },
    ];

    const handleFileClick = (file) => {
        const { path } = router.query;
        const safePath = Array.isArray(path) ? path : [];

        const newPath = `/blob/${safePath.join("/")}/${file.name}`;
        router.push(newPath);
    };

    const handleDirectoryClick = async (directory) => {
        const { path } = router.query;
        var newPath = '';
        if (Array.isArray(path)) {
            newPath = `/tree/${path.join("/")}/${directory.name}`;
        } else {
            newPath = `/tree/${directory.name}`;
        }
        router.push({
            pathname: newPath,
        });
    };

    const handleGoBack = () => {
        const { path } = router.query;
        const safePath = Array.isArray(path) ? path : [];

        if (safePath.length == 1) {
            router.push('/')
        } else {
            router.push(`/tree/${safePath.slice(0, -1).join('/')}`);
        }
    };

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

    const handleOk = () => {
        setConfirmLoading(true);
        setTimeout(() => {
            setOpen(false);
            setConfirmLoading(false);
        }, 2000);
    };

    const handleCancel = () => {
        setOpen(false);
    };

    return (
        <div className={styles.dirTable} style={fileCodeContainerStyle}>
            <Table style={{ clear: "none" }} rowClassName={styles.dirShowTr} pagination={false} columns={columns} dataSource={sortedDir} />
            <Modal
                title="Did you want to publish repo to public?"
                open={open}
                onOk={handleOk}
                confirmLoading={confirmLoading}
                onCancel={handleCancel}
            >
                <Input showCount maxLength={20} placeholder={modalText} />
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

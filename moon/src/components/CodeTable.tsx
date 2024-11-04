'use client'

import 'github-markdown-css/github-markdown-light.css'
import { usePathname, useRouter } from 'next/navigation'
import Markdown from 'react-markdown'
import { formatDistance, fromUnixTime } from 'date-fns'
import styles from './CodeTable.module.css'
import { Space, Table, TableProps } from 'antd/lib'
import {
    FolderIcon,
    DocumentIcon,
} from '@heroicons/react/20/solid'

export interface DataType {
    oid: string;
    name: string;
    content_type: string;
    message: string;
    date: number;
}

const CodeTable = ({ directory, readmeContent }) => {
    const router = useRouter();
    const fileCodeContainerStyle = {
        width: '100%',
        margin: '0 auto',
        borderRadius: '0.5rem',
        marginTop: '10px'
    };
    const pathname = usePathname();
    let real_path = pathname.replace("/tree", "");
    var columns: TableProps<DataType>['columns'] = [
        {
            title: 'Name',
            dataIndex: ['name', 'content_type'],
            key: 'name',
            render: (_, record) => {
                return <>
                    <Space>
                        {record.content_type === "directory" && <FolderIcon className="size-6" />}
                        {record.content_type === "file" && <DocumentIcon className="size-6" />}
                        <a>{record.name}</a>
                    </Space>
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
        }
    ];
    const handleRowClick = (record) => {
        if (record.content_type === "file") {
            const newPath = `/blob/${real_path}/${record.name}`;
            router.push(newPath);
        } else {
            var newPath = '';
            if (real_path === '/') {
                newPath = `/tree/${record.name}`;
            } else {
                newPath = `/tree/${real_path}/${record.name}`;
            }
            router.push(
                newPath,
            );
        }
    }

    const handleGoBack = () => {
        const safePath = real_path.split('/');

        if (safePath.length == 1) {
            router.push('/')
        } else {
            router.push(`/tree/${safePath.slice(0, -1).join('/')}`);
        }
    };

    return (
        <div style={fileCodeContainerStyle}>
            <Table style={{ clear: "none" }} rowClassName={styles.dirShowTr}
                pagination={false} columns={columns}
                dataSource={directory} 
                rowKey="name"
                onRow={(record) => {
                    return {
                        onClick: (event) => { handleRowClick(record) }
                    };
                }}
            />
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

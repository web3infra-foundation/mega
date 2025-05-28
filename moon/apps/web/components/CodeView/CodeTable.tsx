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
import { useMemo } from 'react';

export interface DataType {
    oid: string;
    name: string;
    content_type: string;
    message: string;
    date: number;
}

const CodeTable = ({ directory, readmeContent}:any) => {
    const router = useRouter();
    const pathname = usePathname();
    let real_path = pathname?.replace("/tree", "");
    
    const columns = useMemo<TableProps<DataType>['columns']>(() => [
        {
          title: 'Name',
          dataIndex: ['name', 'content_type'],
          key: 'name',
          render: (_, record) => (
            <Space>
              {record.content_type === "directory" && <FolderIcon className="size-6" />}
              {record.content_type === "file" && <DocumentIcon className="size-6" />}
              <a>{record.name}</a>
            </Space>
          )
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
    ], []);
    
    const handleRowClick = (record: DataType) => {
    const normalizedPath = real_path?.replace(/^\/|\/$/g, '');
    const pathParts = normalizedPath?.split('/') || [];
  
    if (record.content_type === "file") {
      const newPath = `/blob/${normalizedPath}/${encodeURIComponent(record.name)}`;

      router.push(newPath);
    } else {
      let newPath: string;
      
      const hasTree = pathParts?.includes('tree');
      
      if (!hasTree && pathParts.length >= 2) {
        pathParts?.splice(2, 0, 'tree'); 
      }
      pathParts?.push(encodeURIComponent(record.name));
      
      newPath = `/${pathParts?.join('/')}`;
      router.push(newPath);
    }
    }

    return (
        <div>
            <Table style={{ clear: "none" }} rowClassName={styles.dirShowTr}
                pagination={false} columns={columns}
                dataSource={directory} 
                rowKey="name"
                onRow={(record) => {
                    return {
                        onClick: () => { handleRowClick(record) }
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

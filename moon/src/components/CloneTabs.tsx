import React, { useEffect, useState } from 'react';
import { Tabs, TabsProps, Button, Space, Popover, Input } from 'antd';
import {
    CodeBracketIcon,
} from '@heroicons/react/16/solid'
import copy from 'copy-to-clipboard';
import { CopyOutlined, CheckOutlined, DownloadOutlined } from '@ant-design/icons';
import { usePathname } from 'next/navigation';


const CloneTabs: React.FC = () => {
    const pathname = usePathname();
    const [text, setText] = useState<string>(pathname);
    const [copied, setCopied] = useState<boolean>(false);
    const [active_tab, setActiveTab] = useState<string>('1')

    const onChange = (key: string) => {
        setActiveTab(key)
    };

    useEffect(() => {
        if (typeof window !== 'undefined') {
            const domain = window.location.origin;
            if (active_tab === '1') {
                setText(`${domain}${pathname.replace('/tree', '')}.git`);
            } else {
                setText(`ssh://git@${window.location.hostname}:${pathname.replace('/tree', '')}.git`);
            }
        }
    }, [pathname, active_tab]);



    const handleCopy = () => {
        copy(text);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000); // Reset after 2 seconds
    };

    const tab_items: TabsProps['items'] = [
        {
            key: '1',
            label: 'HTTP',
            children:
                <Space style={{ width: '100%' }}>
                    <Input value={text} />
                    <Button onClick={handleCopy} icon={copied ? <CheckOutlined /> : <CopyOutlined />} size={'small'} />
                </Space>
        },
        {
            key: '2',
            label: 'SSH',
            children: <Space style={{ width: '100%' }}>
                <Input value={text} />
                <Button onClick={handleCopy} icon={copied ? <CheckOutlined /> : <CopyOutlined />} size={'small'} />
            </Space>
        }
    ];

    return (
        <Popover placement="bottomRight"
            content={<Tabs defaultActiveKey="1" items={tab_items} onChange={onChange} />}
            trigger="click">
            <Button icon={<DownloadOutlined />}>Code</Button>
        </Popover>
    )

}

export default CloneTabs;

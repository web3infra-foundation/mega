'use client'

import { Heading } from '@/components/catalyst/heading'
import React, { useCallback, useEffect, useState } from 'react';
import { Flex, List, PaginationProps, Tag, Button, Tabs, TabsProps } from 'antd';
import { formatDistance, fromUnixTime } from 'date-fns';
import { CheckCircleOutlined, ExclamationCircleOutlined } from '@ant-design/icons';
import Link from 'next/link';

interface Item {
    link: string,
    title: string,
    status: string,
    open_timestamp: number,
    merge_timestamp: number | null,
    updated_at: number,
}

export default function IssuePage() {
    const [itemList, setItemList] = useState<Item[]>([]);
    const [numTotal, setNumTotal] = useState(0);
    const [pageSize, setPageSize] = useState(10);
    const [status, setStatus] = useState("open")

    const fetchData = useCallback(async (page: number, per_page: number) => {
        try {
            const res = await fetch(`/api/issue/list`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    pagination: {
                        page: page,
                        per_page: per_page
                    },
                    additional: {
                        status: status
                    }
                }),
            });
            const response = await res.json();
            const data = response.data.data;
            setItemList(data.items);
            setNumTotal(data.total)
        } catch (error) {
            console.error('Error fetching data:', error);
        }
    }, [status]);

    useEffect(() => {
        fetchData(1, pageSize);
    }, [pageSize, status, fetchData]);

    const getStatusTag = (status: string) => {
        switch (status) {
            case 'open':
                return <Tag color="error">open</Tag>;
            case 'closed':
                return <Tag color="success">closed</Tag>;
        }
    };

    const getStatusIcon = (status: string) => {
        switch (status) {
            case 'open':
                return <ExclamationCircleOutlined />;
            case 'closed':
                return <CheckCircleOutlined />;
        }
    };

    const getDescription = (item: Item) => {
        switch (item.status) {
            case 'open':
                return `Issue opened by Admin ${formatDistance(fromUnixTime(item.open_timestamp), new Date(), { addSuffix: true })} `;
            case 'closed':
                return (`Issue ${item.link} closed by Admin ${formatDistance(fromUnixTime(item.updated_at), new Date(), { addSuffix: true })}`)
        }
    }

    const onChange: PaginationProps['onChange'] = (current, pageSize) => {
        fetchData(current, pageSize);
    };

    const tabsChange = (activeKey: string) => {
        if (activeKey === '1') {
            setStatus("open");
        } else {
            setStatus("closed");
        }
    }

    const tab_items: TabsProps['items'] = [
        {
            key: '1',
            label: 'Open',
        },
        {
            key: '2',
            label: 'Closed',
        }
    ];

    return (
        <>
            <Heading>Issues</Heading>
            <Flex justify={'flex-end'} >
                <Button style={{ backgroundColor: '#428646' }} href='/issue/new'>New Issue</Button>
            </Flex>

            <Tabs defaultActiveKey="1" items={tab_items} onChange={tabsChange} />

            <List
                style={{ width: '80%', marginLeft: '10%', marginTop: '10px' }}
                pagination={{ align: "center", pageSize: pageSize, total: numTotal, onChange: onChange }}
                dataSource={itemList}
                renderItem={(item, index) => (
                    <List.Item>
                        <List.Item.Meta
                            avatar={
                                // <ExclamationCircleOutlined />
                                getStatusIcon(item.status)
                            }
                            title={<Link href={`/issue/${item.link}`}>{item.title} {getStatusTag(item.status)}</Link>}
                            description={getDescription(item)}
                        />
                    </List.Item>
                )}
            />
        </>
    )
}
'use client'

import React, { useCallback, useEffect, useState } from 'react';
import { List, PaginationProps, Tag, Tabs, TabsProps } from 'antd';
import { formatDistance, fromUnixTime } from 'date-fns';
import { MergeOutlined, PullRequestOutlined, CloseCircleOutlined } from '@ant-design/icons';
import Link from 'next/link';
import { Heading } from '@/components/catalyst/heading';

interface MrInfoItem {
    link: string,
    title: string,
    status: string,
    open_timestamp: number,
    merge_timestamp: number | null,
    updated_at: number,
}

export default function MergeRequestPage() {
    const [mrList, setMrList] = useState<MrInfoItem[]>([]);
    const [numTotal, setNumTotal] = useState(0);
    const [pageSize, setPageSize] = useState(10);
    const [status, setStatus] = useState('open')

    const fetchData = useCallback(async (page: number, per_page: number) => {
        try {
            const res = await fetch(`/api/mr/list`, {
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
            setMrList(data.items);
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
                return <Tag color="success">open</Tag>;
            case 'merged':
                return <Tag color="purple">merged</Tag>;
            case 'closed':
                return <Tag color="error">closed</Tag>;
        }
    };

    const getStatusIcon = (status: string) => {
        switch (status) {
            case 'open':
                return <PullRequestOutlined />;
            case 'closed':
                return <CloseCircleOutlined />;
            case 'merged':
                return <MergeOutlined />;
        }
    };

    const getDescription = (item: MrInfoItem) => {
        switch (item.status) {
            case 'open':
                return `MergeRequest opened by Admin ${formatDistance(fromUnixTime(item.open_timestamp), new Date(), { addSuffix: true })} `;
            case 'merged':
                if (item.merge_timestamp !== null) {
                    return `MergeRequest merged by Admin ${formatDistance(fromUnixTime(item.merge_timestamp), new Date(), { addSuffix: true })}`;
                } else {
                    return "";
                }
            case 'closed':
                return (`MR ${item.link} closed by Admin ${formatDistance(fromUnixTime(item.updated_at), new Date(), { addSuffix: true })}`)
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

    <PullRequestOutlined />

    return (
        <>
            <Heading>Merge Request</Heading>
            <br />
            <Tabs defaultActiveKey="1" items={tab_items} onChange={tabsChange} />

            <List
                className="w-full mt-2"
                pagination={{ align: "center", pageSize: pageSize, total: numTotal, onChange: onChange }}
                dataSource={mrList}
                renderItem={(item, index) => (
                    <List.Item>
                        <List.Item.Meta
                            avatar={
                                getStatusIcon(item.status)
                            }
                            title={<Link href={`/mr/${item.link}`}>{`${item.title}`} {getStatusTag(item.status)}</Link>}
                            description={getDescription(item)}
                        />
                    </List.Item>
                )}
            />
        </>
    )
}
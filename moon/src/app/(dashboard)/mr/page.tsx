'use client'
import { useEffect, useState } from 'react';
import React from 'react';
import { List, PaginationProps, Tag } from 'antd/lib';
import { format, formatDistance, fromUnixTime } from 'date-fns'
import { MergeOutlined } from '@ant-design/icons';
import Link from 'next/link';


interface MrInfoItem {
    mr_link: string,
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

    const fetchData = async (page: number, per_page: number) => {
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
                        status: ""
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
    };

    useEffect(() => {

        fetchData(1, pageSize);
    }, [pageSize]);

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
                return (`MR ${item.mr_link} closed by Admin ${formatDistance(fromUnixTime(item.updated_at), new Date(), { addSuffix: true })}`)
        }
    }

    const onChange: PaginationProps['onChange'] = (current, pageSize) => {
        fetchData(current, pageSize);
    };

    return (
        <List
            style={{ width: '80%', marginLeft: '10%', marginTop: '10px' }}
            pagination={{ align: "center", pageSize: pageSize, total: numTotal, onChange: onChange }}
            dataSource={mrList}
            renderItem={(item, index) => (
                <List.Item>
                    <List.Item.Meta
                        avatar={
                            <MergeOutlined twoToneColor="#eb2f96" />
                        }
                        title={<Link href={`/mr/${item.mr_link}`}>{`MR ${item.mr_link} open by Mega automacticlly${item.title}`}{getStatusTag(item.status)}</Link>}
                        description={getDescription(item)}
                    />
                </List.Item>
            )}
        />
    )
}
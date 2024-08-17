import React from 'react';
import { Avatar, List, Tag } from 'antd/lib';
import { format, formatDistance, fromUnixTime } from 'date-fns'
import { MergeOutlined } from '@ant-design/icons';
import Link from 'next/link';

interface MrInfoItem {
    id: number,
    title: string,
    status: string,
    open_timestamp: number,
    merge_timestamp: number | null,
}

interface MergeListProps {
    mrList: MrInfoItem[];
}

const MergeList: React.FC<MergeListProps> = ({ mrList }) => {

    const getStatusTag = (status: string) => {
        switch (status) {
            case 'open':
                return <Tag color="success">open</Tag>;
            case 'merged':
                return <Tag color="purple">merged</Tag>;
            case 'closed':
                return <Tag color="failed">closed</Tag>;
        }
    };

    const getDescription = (item: MrInfoItem) => {
        switch (item.status) {
            case 'open':
                return `MergeRequest opened on ${format(fromUnixTime(Number(item.open_timestamp)), 'MMM d')} by Admin`;
            case 'merged':
                if (item.merge_timestamp !== null) {
                    return `MergeRequest by Admin was merged ${formatDistance(fromUnixTime(item.merge_timestamp), new Date(), { addSuffix: true })}`;
                } else {
                    return "";
                }
            case 'closed':
                return <Tag color="failed">closed</Tag>;
        }
    }

    return (
        <List
            style={{ width: '90%', marginLeft: '5%', marginTop: '10px' }}
            pagination={{ align: "center" }}
            dataSource={mrList}
            renderItem={(item, index) => (
                <List.Item>
                    <List.Item.Meta
                        avatar={
                            <MergeOutlined twoToneColor="#eb2f96" />
                        }
                        title={<Link href={`/mr?id=${item.id}`}>{`MR ${item.id} open by Mega automacticlly${item.title}`}{getStatusTag(item.status)}</Link>}
                        description={getDescription(item)}
                    />
                </List.Item>
            )}
        />
    );
};

export default MergeList;
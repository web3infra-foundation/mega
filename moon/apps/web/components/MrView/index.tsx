'use client'

import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { List, PaginationProps, Tag, Tabs, TabsProps } from 'antd';
import { formatDistance, fromUnixTime } from 'date-fns';
import { ChevronSelectIcon,AlarmIcon,ClockIcon} from '@gitmono/ui/Icons'
import { Link } from '@gitmono/ui/Link'
import { Heading } from './catalyst/heading';
import { usePostMrList } from '@/hooks/usePostMrList';
import { apiErrorToast } from '@/utils/apiErrorToast'
import { useScope } from '@/contexts/scope'

interface MrInfoItem {
    link: string,
    title: string,
    status: string,
    open_timestamp: number,
    merge_timestamp: number | null,
    updated_at: number,
}

export default function MrView() {
    const [mrList, setMrList] = useState<MrInfoItem[]>([]);
    const [numTotal, setNumTotal] = useState(0);
    const [pageSize] = useState(10);
    const [status, setStatus] = useState('open')
    const [page, setPage] = useState(1);
    const [isLoading, setIsLoading] = useState(false);
    const { scope } = useScope()
    const { mutate: fetchMrList } = usePostMrList();

    const loadMrList = useCallback(() => {
        setIsLoading(true);
        fetchMrList(
            {
            data: {
                pagination: {
                page,
                per_page: pageSize
                },
                additional: {
                status
                }
            }
            },
            {
            onSuccess: (response) => {
                const data = response.data;

                setMrList(
                data?.items?.map(item => ({
                    ...item,
                    merge_timestamp: item.merge_timestamp ?? null
                })) ?? []
                );
                setNumTotal(data?.total ?? 0);
            },
            onError: apiErrorToast,
            onSettled: () => setIsLoading(false)
            }
        );
    }, [page, pageSize, status, fetchMrList]);

    useEffect(() => {
      loadMrList();
    }, [loadMrList]);

    const getStatusTag = (status: string) => {
        const normalizedStatus = status.toLowerCase();

        switch (normalizedStatus) {
            case 'open':
                return <Tag color="success">open</Tag>;
            case 'merged':
                return <Tag color="purple">merged</Tag>;
            case 'closed':
                return <Tag color="error">closed</Tag>;
            default:
                 return null;
        }
    };

    const getStatusIcon = (status: string) => {
        const normalizedStatus = status.toLowerCase();

        switch (normalizedStatus) {
            case 'open':
                return <ChevronSelectIcon />;
            case 'closed':
                return <AlarmIcon />;
            case 'merged':
                return <ClockIcon />;
            default:
                 return null;
        }
    };

    const getDescription = (item: MrInfoItem) => {
        const normalizedStatus = item.status.toLowerCase();

        switch (normalizedStatus) {
            case 'open':
                return `MergeRequest opened by Admin ${formatDistance(fromUnixTime(item.open_timestamp), new Date(), { addSuffix: true })} `;
            case 'merged':
                if (item.merge_timestamp !== null) {
                    return `MergeRequest merged by Admin ${formatDistance(fromUnixTime(item.merge_timestamp), new Date(), { addSuffix: true })}`;
                } else {
                    return "";
                }
            case 'closed':
                return (`MR ${item.link} closed by Admin ${formatDistance(fromUnixTime(item.updated_at), new Date(), { addSuffix: true })}`);
            default:
                 return null;
        }
    }

    const onChange: PaginationProps['onChange'] = (current) => {
        setPage(current);
    };

    const tabsChange = (activeKey: string) => {
        if (activeKey === '1') {
            setStatus("open");
        } else {
            setStatus("closed");
        }
    }

    const tab_items = useMemo(() =>  [
        {
            key: '1',
            label: 'Open',
        },
        {
            key: '2',
            label: 'Closed',
        }
    ], []) as TabsProps['items'];

    return (
        <div className="m-4">
            <Heading>Merge Request</Heading>
            <br />
            <Tabs defaultActiveKey="1" items={tab_items} onChange={tabsChange} />

            <List
                className="w-full mt-2"
                pagination={{ align: "center", pageSize: pageSize, total: numTotal, onChange: onChange }}
                dataSource={mrList}
                loading={isLoading}
                renderItem={(item) => (
                    <List.Item>
                        <List.Item.Meta
                            avatar={
                                getStatusIcon(item.status)
                            }
                            title={
                                <Link href={{
                                pathname: `/${scope}/mr/${item.link}`,
                                query: {
                                    title: item.title,
                                }
                                }}>
                                {`${item.title}`} {getStatusTag(item.status)}
                                </Link>
                            }
                            description={getDescription(item)}
                        />
                    </List.Item>
                )}
            />
        </div>
    )
}
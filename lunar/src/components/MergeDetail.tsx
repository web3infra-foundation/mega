'use client'
import React, { useEffect, useState } from 'react';
import { Card, Button, List } from 'antd/lib';
import { useRouter } from 'next/navigation';
import { useMRFiles } from '@/app/api/fetcher';

const endpoint = process.env.NEXT_PUBLIC_API_URL;


const MRDetailPage = ({ mrDetail }) => {
    const router = useRouter();
    const [filedata, setFileData] = useState([]);
    const [loadings, setLoadings] = useState<boolean[]>([]);
    const [error, setError] = useState(null);

    const { mrFiles, isMRLoading, isMRError } = useMRFiles(mrDetail.id);

    useEffect(() => {
        if (isMRLoading) {
            set_to_loading(2)
        }
        if (mrFiles) {
            setFileData(mrFiles.data);
            cancel_loading(2)
        }
    }, [mrDetail, mrFiles, isMRLoading]);


    const set_to_loading = (index: number) => {
        setLoadings((prevLoadings) => {
            const newLoadings = [...prevLoadings];
            newLoadings[index] = true;
            return newLoadings;
        });
    }

    const cancel_loading = (index: number) => {
        setLoadings((prevLoadings) => {
            const newLoadings = [...prevLoadings];
            newLoadings[index] = false;
            return newLoadings;
        });
    }

    const approve_mr = async (index: number, id: number) => {
        set_to_loading(index);
        const res = await fetch(`${endpoint}/api/v1/mono/mr/${id}/merge`,{
            method: 'POST',
        });
        if (res) {
            cancel_loading(index);
        }
        
        if (res.ok) {
            router.refresh();
        }
    };


    return (
        <Card title="Merge Request Detail Page">
            {mrDetail.status === "open" &&
                <Button
                    type="primary"
                    loading={loadings[1]}
                    onClick={() => approve_mr(1, mrDetail.id)}
                >
                    Merge MR
                </Button>
            }

            <Card
                style={{ marginTop: 16 }}
                type="inner"
                title={mrDetail.id}
                extra={<a href="#">More</a>}
            >
                <List
                    style={{ width: '30%' }}
                    header={<div>Change File List</div>}
                    bordered
                    dataSource={filedata}
                    loading = {loadings[2]}
                    renderItem={(item) => (
                        <List.Item>
                            {item}
                        </List.Item>
                    )}
                />
            </Card>
        </Card>
    )
}

export default MRDetailPage;
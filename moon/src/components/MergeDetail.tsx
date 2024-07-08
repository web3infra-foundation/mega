import React, { useEffect, useState } from 'react';
import { Card, Button, List, Typography } from 'antd/lib';
import { useRouter } from 'next/router';

const MRDetailPage = ({ mrDetail }) => {
    const router = useRouter();
    const [filedata, setFileData] = useState([]);
    const [loadings, setLoadings] = useState<boolean[]>([]);
    const [error, setError] = useState(null);

    useEffect(() => {
        const fetchFileList = async () => {
            set_to_loading(2)
            try {
                const res = await fetch(`/api/mr/files?id=${mrDetail.id}`);
                const result = await res.json();

                setFileData(result.data);
            } catch (err) {
                setError(err);
            } finally {
                cancel_loading(2)
            }
        };
        fetchFileList();
    }, [mrDetail]);


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

    const enterLoading = async (index: number, id: number) => {
        set_to_loading(index);
        const res = await fetch(`/api/mr/merge?id=${id}`);
        if (res) {
            cancel_loading(index);
        }
        if (res.ok) {
            router.reload();
        }
    };


    return (
        <Card title="Card title">
            {mrDetail.status === "open" &&
                <Button
                    type="primary"
                    loading={loadings[1]}
                    onClick={() => enterLoading(1, mrDetail.id)}
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
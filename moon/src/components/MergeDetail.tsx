import React, { useState } from 'react';
import { Card, Button } from 'antd/lib';
import { useRouter } from 'next/router';

const MRDetailPage = ({ mrDetail }) => {
    const router = useRouter();
    const [loadings, setLoadings] = useState<boolean[]>([]);

    const enterLoading = async (index: number, id: number) => {
        setLoadings((prevLoadings) => {
            const newLoadings = [...prevLoadings];
            newLoadings[index] = true;
            return newLoadings;
        });
        const res = await fetch(`/api/mr/merge?id=${id}`);
        if (res) {
            setLoadings((prevLoadings) => {
                const newLoadings = [...prevLoadings];
                newLoadings[index] = false;
                return newLoadings;
            });
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
                {mrDetail.status}
            </Card>
        </Card>
    )
}

export default MRDetailPage;
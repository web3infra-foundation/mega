'use client'
import MergeList from "@/components/MergeList";
import { useEffect, useState } from 'react';

export default function MergeRequestPage() {
    const [mrList, setMrList] = useState([]);
    useEffect(() => {
        const fetchData = async () => {
            try {
                const res = await fetch(`/api/mr/list?status=`);
                const response = await res.json();
                const mrList = response.data.data;
                setMrList(mrList);
            } catch (error) {
                console.error('Error fetching data:', error);
            }
        };
        fetchData();
    }, []);
    return (
        <div>
            <MergeList mrList={mrList} />
        </div>
    )
}
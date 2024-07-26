'use client'

import React, { useEffect, useState } from 'react'
import DataList from '@/components/DataList'


export default function HomePage() {
  const [repo_list, setRepoList] = useState([]);

  useEffect(() => {
    const fetchData = async () => {
      try {
        let repo_list = await getRepoList();
        setRepoList(repo_list);
      } catch (error) {
        console.error('Error fetching data:', error);
      }
    };
    fetchData();
  }, []);

  return (
    <div>
      <DataList data={repo_list} />
    </div>
  )
}

async function getRepoList() {
  const res = await fetch(`api/relay/repo_list`);
  const response = await res.json();
  const repo_list = response.data;
  return repo_list
}
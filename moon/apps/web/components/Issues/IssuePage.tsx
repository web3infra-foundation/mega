'use client'

import React, { useCallback, useEffect, useState } from 'react'
// import { CheckCircleOutlined, ExclamationCircleOutlined } from '@ant-design/icons'
import { Link } from '@gitmono/ui'
import { Button, Flex, List, PaginationProps, Tabs, TabsProps, Tag } from 'antd'
import { formatDistance, fromUnixTime } from 'date-fns'
import { useRouter } from 'next/router'

import { Heading } from '@/components/Catalyst/Heading'
import { useGetIssueLists } from '@/hooks/issues/useGetIssueLists'
import { apiErrorToast } from '@/utils/apiErrorToast'

interface Item {
  closed_at?: number | null
  link: string
  owner: number
  title: string
  status: string
  open_timestamp: number
  updated_at: number
}

export default function IssuePage() {
  const router = useRouter()
  const [itemList, setItemList] = useState<Item[]>([])
  const [numTotal, setNumTotal] = useState(0)
  const [pageSize, _setPageSize] = useState(10)
  const [loading, setLoading] = useState(false)
  const [status, setStatus] = useState('open')
  const { mutate: issueLists } = useGetIssueLists()

  const fetchData = useCallback(
    (page: number, per_page: number) => {
      setLoading(true)

      issueLists(
        {
          data: { pagination: { page, per_page }, additional: { status } }
        },
        {
          onSuccess: (response) => {
            const data = response.data

            setItemList(data?.items ?? [])
            setNumTotal(data?.total ?? 0)
          },
          onError: apiErrorToast,
          onSettled: () => setLoading(false)
        }
      )
    },

    [status, issueLists]
  )

  useEffect(() => {
    fetchData(1, pageSize)
  }, [pageSize, fetchData])

  const getStatusTag = (status: string) => {
    switch (status) {
      case 'open':
        return <Tag color='error'>open</Tag>
      case 'closed':
        return <Tag color='success'>closed</Tag>
    }
  }

  // const getStatusIcon = (status: string) => {
  //   switch (status) {
  //     case 'open':
  //     // return <ExclamationCircleOutlined />
  //     case 'closed':
  //     // return <CheckCircleOutlined />
  //   }
  // }

  const getDescription = (item: Item) => {
    switch (item.status) {
      case 'open':
        return `Issue opened by Admin ${formatDistance(fromUnixTime(item.open_timestamp), new Date(), { addSuffix: true })} `
      case 'closed':
        return `Issue ${item.link} closed by Admin ${formatDistance(fromUnixTime(item.updated_at), new Date(), { addSuffix: true })}`
    }
  }

  const onChange: PaginationProps['onChange'] = (current, pageSize) => {
    fetchData(current, pageSize)
  }

  const tabsChange = (activeKey: string) => {
    if (activeKey === '1') {
      setStatus('open')
    } else {
      setStatus('closed')
    }
  }

  const tab_items: TabsProps['items'] = [
    {
      key: '1',
      label: 'Open'
    },
    {
      key: '2',
      label: 'Closed'
    }
  ]

  return (
    <>
      <div className='container p-10'>
        <Heading>Issues</Heading>
        <Flex justify={'flex-end'}>
          <Button
            style={{ backgroundColor: '#428646', color: '#fff' }}
            onClick={() => router.push(`/${router.query.org}/issue/new`)}
          >
            New Issue
          </Button>
        </Flex>

        <Tabs defaultActiveKey='1' items={tab_items} onChange={tabsChange} />

        <List
          style={{ width: '80%', marginLeft: '10%', marginTop: '10px' }}
          pagination={{ align: 'center', pageSize: pageSize, total: numTotal, onChange: onChange }}
          dataSource={itemList}
          loading={loading}
          renderItem={(item, _index) => (
            <List.Item>
              <List.Item.Meta
                // avatar={
                //   // <ExclamationCircleOutlined />
                //   getStatusIcon(item.status)
                // }
                title={
                  <Link href={`/${router.query.org}/issue/${item.link}`}>
                    {item.title} {getStatusTag(item.status)}
                  </Link>
                }
                description={getDescription(item)}
              />
            </List.Item>
          )}
        />
      </div>
    </>
  )
}

'use client'

import React from 'react'
import { Pagination } from '@primer/react'
import Head from 'next/head'

import { UIText } from '@gitmono/ui'

import { AppLayout } from '@/components/Layout/AppLayout'
import { ClientsTable, deriveStatus, OrionClient, OrionClientStatus } from '@/components/OrionClient'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { PageWithLayout } from '@/utils/types'

const OrionClientPage: PageWithLayout<any> = () => {
  const mockClients = React.useMemo<OrionClient[]>(
    () => [
      {
        client_id: 'worker-1',
        hostname: 'build-node-01',
        instance_id: 'i-09f3a1b2c3',
        orion_version: '0.3.1',
        start_time: '2025-12-17T09:23:11Z',
        last_heartbeat: '2025-12-20T09:22:50Z',
        status: 'running'
      },
      {
        client_id: 'worker-2',
        hostname: 'build-node-02',
        instance_id: 'i-09f3a1b2c4',
        orion_version: '0.3.1',
        start_time: '2025-12-18T11:11:00Z',
        last_heartbeat: '2025-12-20T09:22:00Z',
        status: 'busy'
      },
      {
        client_id: 'worker-3',
        hostname: 'build-node-03',
        instance_id: 'pod-009',
        orion_version: '0.3.0',
        start_time: '2025-12-17T08:00:00Z',
        last_heartbeat: '2025-12-20T09:21:10Z',
        status: 'idle'
      },
      {
        client_id: 'worker-4',
        hostname: 'build-node-04',
        instance_id: 'pod-010',
        orion_version: '0.3.0',
        start_time: '2025-12-17T08:00:00Z',
        last_heartbeat: '2025-12-20T09:20:00Z',
        status: 'error'
      },
      {
        client_id: 'worker-5',
        hostname: 'build-node-05',
        instance_id: 'pod-011',
        orion_version: '0.2.9',
        start_time: '2025-12-17T08:00:00Z',
        last_heartbeat: '2025-12-20T09:19:10Z',
        status: 'preparing'
      },
      {
        client_id: 'worker-6',
        hostname: 'build-node-06',
        instance_id: 'pod-012',
        orion_version: '0.2.9',
        start_time: '2025-12-17T08:00:00Z',
        // no status -> derive from heartbeat, will become offline if >30s
        last_heartbeat: '2025-12-20T09:17:00Z'
      },
      {
        client_id: 'worker-7',
        hostname: 'build-node-07',
        instance_id: 'pod-013',
        orion_version: '0.3.1',
        start_time: '2025-12-19T03:10:00Z',
        last_heartbeat: '2025-12-20T09:22:10Z',
        status: 'downloading'
      },
      {
        client_id: 'worker-8',
        hostname: 'build-node-08',
        instance_id: 'pod-014',
        orion_version: '0.3.1',
        start_time: '2025-12-19T03:12:00Z',
        last_heartbeat: '2025-12-20T09:21:40Z',
        status: 'uploading'
      },
      {
        client_id: 'worker-9',
        hostname: 'build-node-09',
        instance_id: 'pod-015',
        orion_version: '0.3.1',
        start_time: '2025-12-19T03:15:00Z',
        last_heartbeat: '2025-12-19T01:10:03Z',
        status: 'idle'
      }
    ],
    []
  )

  const [searchQuery, setSearchQuery] = React.useState<string>('')
  const [statusFilter, setStatusFilter] = React.useState<OrionClientStatus | 'all'>('all')
  const [currentPage, setCurrentPage] = React.useState<number>(1)

  const perPage = 8

  const filtered = React.useMemo(() => {
    return mockClients.filter((c) => {
      const status = deriveStatus(c)
      const matchStatus = statusFilter === 'all' ? true : status === statusFilter

      const text = searchQuery.trim().toLowerCase()
      const matchText =
        text === '' || c.client_id.toLowerCase().includes(text) || c.hostname.toLowerCase().includes(text)

      return matchStatus && matchText
    })
  }, [mockClients, searchQuery, statusFilter])

  const pageCount = React.useMemo(() => {
    return Math.max(1, Math.ceil(filtered.length / perPage))
  }, [filtered.length])

  React.useEffect(() => {
    setCurrentPage(1)
  }, [searchQuery, statusFilter])

  React.useEffect(() => {
    setCurrentPage((p) => Math.min(Math.max(1, p), pageCount))
  }, [pageCount])

  const pagedClients = React.useMemo(() => {
    const start = (currentPage - 1) * perPage

    return filtered.slice(start, start + perPage)
  }, [currentPage, filtered])

  return (
    <>
      <Head>
        <title>Orion Client</title>
      </Head>
      <div className='flex flex-col gap-4 p-4'>
        <div className='flex flex-col gap-2'>
          <div className='flex flex-wrap items-center justify-between gap-3'>
            <div>
              <h1 className='text-xl font-semibold'>Orion Clients</h1>
              <UIText color='text-muted' size='text-sm'>
                Total clients {filtered.length}
              </UIText>
            </div>
          </div>

          <div className='border-b' />
        </div>

        <div className='group flex min-h-[35px] items-center rounded-md border border-gray-300 bg-white px-3 shadow-sm transition-all focus-within:border-blue-500 focus-within:shadow-md focus-within:ring-2 focus-within:ring-blue-100 hover:border-gray-400 dark:border-gray-700 dark:bg-gray-900 dark:hover:border-gray-500'>
          <div className='flex items-center text-gray-400'>
            <svg
              xmlns='http://www.w3.org/2000/svg'
              className='h-4 w-4'
              fill='none'
              viewBox='0 0 24 24'
              stroke='currentColor'
            >
              <path
                strokeLinecap='round'
                strokeLinejoin='round'
                strokeWidth='2'
                d='M21 21l-4.35-4.35M11 19a8 8 0 100-16 8 8 0 000 16z'
              />
            </svg>
          </div>
          <input
            type='text'
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder='Search by Client ID / Hostname'
            className='w-full flex-1 border-none bg-transparent text-sm text-gray-700 outline-none ring-0 placeholder:text-gray-400 focus:outline-none focus:ring-0 dark:text-gray-100 dark:placeholder:text-gray-500'
          />
        </div>

        <ClientsTable
          clients={pagedClients}
          statusFilter={statusFilter}
          onStatusChange={(value: OrionClientStatus | 'all') => setStatusFilter(value)}
          statusOptions={[
            { value: 'all', label: 'All statuses' },
            { value: 'idle', label: 'Idle' },
            { value: 'busy', label: 'Busy' },
            { value: 'downloading', label: 'Downloading source' },
            { value: 'preparing', label: 'Preparing environment' },
            { value: 'running', label: 'Running build' },
            { value: 'uploading', label: 'Uploading artifacts' },
            { value: 'error', label: 'Error' },
            { value: 'offline', label: 'Lost / Offline' }
          ]}
        />

        {pageCount > 1 ? (
          <div className='flex w-full justify-center pt-2'>
            <Pagination
              pageCount={pageCount}
              currentPage={currentPage}
              showPages={{ narrow: false }}
              onPageChange={(_e: any, page: number) => setCurrentPage(page)}
            />
          </div>
        ) : null}
      </div>
    </>
  )
}

OrionClientPage.getProviders = (page: React.ReactElement, pageProps: any) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default OrionClientPage

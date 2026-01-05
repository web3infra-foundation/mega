'use client'

import React from 'react'
import { Pagination } from '@primer/react'
import Head from 'next/head'

import {
  CoreWorkerStatus,
  PageParamsOrionClientQuery,
  PostOrionClientsInfoData,
  TaskPhase
} from '@gitmono/types/generated'
import { Button, UIText } from '@gitmono/ui'
import { RefreshIcon } from '@gitmono/ui/Icons'

import { AppLayout } from '@/components/Layout/AppLayout'
import { ClientsTable, OrionClientStatus } from '@/components/OrionClient'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { usePostOrionClientsInfo } from '@/hooks/OrionClient/OrionClientsInfo'
import { PageWithLayout } from '@/utils/types'

const OrionClientPage: PageWithLayout<any> = () => {
  const [hostnameInput, setHostnameInput] = React.useState<string>('')
  const [debouncedHostname, setDebouncedHostname] = React.useState<string>('')
  const [statusFilter, setStatusFilter] = React.useState<OrionClientStatus | 'all'>('all')
  const [currentPage, setCurrentPage] = React.useState<number>(1)

  const perPage = 8

  const { mutate, isPending, error } = usePostOrionClientsInfo()
  const [clientsPage, setClientsPage] = React.useState<PostOrionClientsInfoData | null>(null)

  React.useEffect(() => {
    const handle = setTimeout(() => {
      setDebouncedHostname(hostnameInput)
    }, 500)

    return () => clearTimeout(handle)
  }, [hostnameInput])

  const requestPayload = React.useMemo<PageParamsOrionClientQuery>(() => {
    const text = debouncedHostname.trim()
    const additional: PageParamsOrionClientQuery['additional'] = {}

    if (text !== '') {
      additional.hostname = text
    }

    if (statusFilter === 'idle') {
      additional.status = CoreWorkerStatus.Idle
    } else if (statusFilter === 'error') {
      additional.status = CoreWorkerStatus.Error
    } else if (statusFilter === 'offline') {
      additional.status = CoreWorkerStatus.Lost
    } else if (statusFilter === 'busy') {
      additional.status = CoreWorkerStatus.Busy
    } else if (statusFilter === 'downloading') {
      additional.status = CoreWorkerStatus.Busy
      additional.phase = TaskPhase.DownloadingSource
    } else if (statusFilter === 'running') {
      additional.status = CoreWorkerStatus.Busy
      additional.phase = TaskPhase.RunningBuild
    }

    return {
      pagination: { page: currentPage, per_page: perPage },
      additional
    }
  }, [currentPage, debouncedHostname, perPage, statusFilter])

  const handleRefresh = React.useCallback(() => {
    mutate(requestPayload, {
      onSuccess: (data) => {
        setClientsPage(data)
      }
    })
  }, [mutate, requestPayload])

  React.useEffect(() => {
    mutate(requestPayload, {
      onSuccess: (data) => {
        setClientsPage(data)
      }
    })
  }, [mutate, requestPayload])

  const total = clientsPage?.total ?? 0

  const pageCount = React.useMemo(() => {
    return Math.max(1, Math.ceil(total / perPage))
  }, [perPage, total])

  React.useEffect(() => {
    setCurrentPage(1)
  }, [hostnameInput, statusFilter])

  React.useEffect(() => {
    setCurrentPage((p) => Math.min(Math.max(1, p), pageCount))
  }, [pageCount])

  const clients = React.useMemo(() => {
    const items = clientsPage?.items ?? []

    return items.map((c) => ({
      client_id: c.client_id,
      hostname: c.hostname,
      orion_version: c.orion_version,
      start_time: c.start_time,
      last_heartbeat: c.last_heartbeat
    }))
  }, [clientsPage])

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
                Total clients {total}
              </UIText>
            </div>
            <Button
              variant='plain'
              iconOnly={<RefreshIcon />}
              accessibilityLabel='Refresh'
              onClick={handleRefresh}
              disabled={isPending}
              tooltip='Refresh'
            />
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
            value={hostnameInput}
            onChange={(e) => setHostnameInput(e.target.value)}
            placeholder='Search by Hostname'
            className='w-full flex-1 border-none bg-transparent text-sm text-gray-700 outline-none ring-0 placeholder:text-gray-400 focus:outline-none focus:ring-0 dark:text-gray-100 dark:placeholder:text-gray-500'
          />
        </div>

        <ClientsTable
          clients={clients}
          isLoading={isPending}
          statusFilter={statusFilter}
          onStatusChange={(value: OrionClientStatus | 'all') => setStatusFilter(value)}
          statusOptions={[
            { value: 'all', label: 'All statuses' },
            { value: 'idle', label: 'Idle' },
            { value: 'busy', label: 'Busy' },
            { value: 'downloading', label: '\u00A0\u00A0Downloading source' },
            { value: 'running', label: '\u00A0\u00A0Running build' },
            { value: 'error', label: 'Error' },
            { value: 'offline', label: 'Lost / Offline' }
          ]}
        />

        {error ? (
          <UIText color='text-muted' size='text-sm'>
            Failed to load Orion clients: {error.message}
          </UIText>
        ) : null}

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

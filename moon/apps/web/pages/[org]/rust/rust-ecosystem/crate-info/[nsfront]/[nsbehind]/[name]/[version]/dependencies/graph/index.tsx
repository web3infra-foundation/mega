'use client'

import React, { useEffect, useState } from 'react'
import { MagnifyingGlassIcon } from '@heroicons/react/24/outline'
import Head from 'next/head'
import { useParams } from 'next/navigation'
import { useRouter } from 'next/router'

import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import DependencyGraph from '@/components/Rust/Graph/DependencyGraph'

import CrateInfoLayout from '../../layout'

interface Deptree {
  name_and_version: string
  cve_count: number
  direct_dependency: Deptree[]
}

const DependenciesGraphPage = () => {
  const params = useParams()
  const router = useRouter()
  const [currentPage, setCurrentPage] = useState(1)
  const [searchTerm, setSearchTerm] = useState('')
  const [graphData, setGraphData] = useState<Deptree | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // 从URL参数中获取crate信息
  const crateName = (params?.name as string) || 'tokio'
  const version = (params?.version as string) || '1.2.01'
  const nsfront = (params?.nsfront as string) || (router.query.org as string)
  const nsbehind = (params?.nsbehind as string) || 'rust/rust-ecosystem/crate-info'

  // 从 API 获取图形数据
  useEffect(() => {
    const fetchGraphData = async () => {
      if (!crateName || !version || !nsfront || !nsbehind) return

      try {
        setLoading(true)
        setError(null)

        const apiBaseUrl = process.env.NEXT_PUBLIC_CRATES_PRO_URL
        const response = await fetch(
          `${apiBaseUrl}/api/crates/${nsfront}/${nsbehind}/${crateName}/${version}/dependencies/graphpage`
        )

        if (!response.ok) {
          throw new Error('Failed to fetch graph data')
        }

        const data: Deptree = await response.json()

        setGraphData(data)
      } catch (err) {
        setError('Failed to load graph data')
      } finally {
        setLoading(false)
      }
    }

    fetchGraphData()
  }, [crateName, version, nsfront, nsbehind])

  const handleBackToTable = () => {
    router.push(
      `/${router.query.org}/rust/rust-ecosystem/crate-info/${nsfront}/${nsbehind}/${crateName}/${version}/dependencies`
    )
  }

  return (
    <>
      <Head>
        <title>Dependencies Graph - {crateName}</title>
      </Head>
      <CrateInfoLayout>
        {/* 主要内容区域 */}
        <div className='flex justify-center'>
          <div className='w-[1370px] px-8 py-4'>
            {/* 统一的白色面板 */}
            <div className='rounded-lg border border-gray-200 bg-white shadow-sm'>
              {/* 搜索和视图切换 - 在面板内部 */}
              <div className='flex items-center justify-between border-b border-gray-200 p-2'>
                <div className='mr-4 flex flex-1 items-center'>
                  <div className='relative ml-2 w-full'>
                    <div className='pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3'>
                      <MagnifyingGlassIcon className='h-5 w-5 text-gray-400' />
                    </div>
                    <input
                      type='text'
                      placeholder='Placeholder'
                      value={searchTerm}
                      onChange={(e) => setSearchTerm(e.target.value)}
                      style={{
                        display: 'flex',
                        height: 'var(--Spacing-8, 36px)',
                        padding: '0 var(--Spacing-1, 4px)',
                        alignItems: 'center',
                        alignSelf: 'stretch',
                        borderRadius: 'var(--Radius-2-max, 4px)',
                        border: '1px solid var(--Colors-Neutral-Neutral-Alpha-5, #0009321f)',
                        background: 'var(--Tokens-Colors-surface, #ffffffe6)',
                        paddingLeft: '40px',
                        width: '100%'
                      }}
                    />
                  </div>
                </div>
                <div className='ml-auto mr-2 flex space-x-2'>
                  <button
                    onClick={handleBackToTable}
                    style={{
                      display: 'flex',
                      height: 'var(--Tokens-Space-button-height-2, 32px)',
                      padding: '0 var(--Spacing-3, 12px)',
                      justifyContent: 'center',
                      alignItems: 'center',
                      gap: 'var(--Spacing-2, 8px)',
                      borderRadius: 'var(--Radius-2-max, 4px)',
                      background: 'var(--Colors-Accent-Accent-Alpha-3, #0047f112)',
                      color: '#002bb7c4',
                      border: '1px solid var(--Colors-Neutral-Neutral-Alpha-5, #0009321f)'
                    }}
                  >
                    <svg className='h-4 w-4' fill='currentColor' viewBox='0 0 20 20'>
                      <path
                        fillRule='evenodd'
                        d='M3 4a1 1 0 011-1h12a1 1 0 011 1v2a1 1 0 01-1 1H4a1 1 0 01-1-1V4zM3 10a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H4a1 1 0 01-1-1v-6zM14 9a1 1 0 00-1 1v6a1 1 0 001 1h2a1 1 0 001-1v-6a1 1 0 00-1-1h-2z'
                        clipRule='evenodd'
                      />
                    </svg>
                    <span>Table</span>
                  </button>
                  <button
                    style={{
                      display: 'flex',
                      height: 'var(--Tokens-Space-button-height-2, 32px)',
                      padding: '0 var(--Spacing-3, 12px)',
                      justifyContent: 'center',
                      alignItems: 'center',
                      gap: 'var(--Spacing-2, 8px)',
                      borderRadius: 'var(--Radius-2-max, 4px)',
                      background: 'var(--Colors-Accent-Accent-9, #3E63DD)',
                      color: 'white',
                      border: 'none'
                    }}
                  >
                    <svg className='h-4 w-4' fill='currentColor' viewBox='0 0 20 20'>
                      <path d='M2 11a1 1 0 011-1h2a1 1 0 011 1v5a1 1 0 01-1 1H3a1 1 0 01-1-1v-5zM8 7a1 1 0 011-1h2a1 1 0 011 1v9a1 1 0 01-1 1H9a1 1 0 01-1-1V7zM14 4a1 1 0 011-1h2a1 1 0 011 1v12a1 1 0 01-1 1h-2a1 1 0 01-1-1V4z' />
                    </svg>
                    <span
                      style={{
                        fontFamily: '"SF Pro"',
                        fontSize: '14px',
                        fontStyle: 'normal',
                        fontWeight: '500',
                        lineHeight: '20px',
                        letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                      }}
                    >
                      Graph
                    </span>
                  </button>
                </div>
              </div>

              {/* 加载状态 */}
              {loading && (
                <div className='flex items-center justify-center py-8'>
                  <div className='text-gray-500'>Loading graph data...</div>
                </div>
              )}

              {/* 错误状态 */}
              {error && (
                <div className='flex items-center justify-center py-8'>
                  <div className='text-red-500'>{error}</div>
                </div>
              )}

              {/* 图形视图内容 */}
              {!loading && !error && graphData && (
                <div className='h-full w-full p-6' style={{ height: '100%', width: '100%' }}>
                  <DependencyGraph data={graphData} />
                </div>
              )}

              {/* 无数据状态 */}
              {!loading && !error && !graphData && (
                <div className='flex items-center justify-center py-8'>
                  <div className='text-gray-500'>No graph data available</div>
                </div>
              )}
            </div>

            {/* 分页功能区 */}
            <div className='mt-8 flex w-full justify-center'>
              <div style={{ width: '1370px', paddingLeft: 32, paddingRight: 32 }}>
                <div className='flex items-center justify-center gap-6' style={{ marginLeft: '-100px' }}>
                  {/* Previous 按钮 */}
                  <button
                    onClick={() => setCurrentPage(Math.max(1, currentPage - 1))}
                    disabled={currentPage === 1}
                    className='flex items-center text-gray-400 hover:text-gray-600 disabled:cursor-not-allowed disabled:opacity-50'
                  >
                    <svg className='mr-1 h-4 w-4' fill='none' stroke='currentColor' viewBox='0 0 24 24'>
                      <path strokeLinecap='round' strokeLinejoin='round' strokeWidth={2} d='M15 19l-7-7 7-7' />
                    </svg>
                    Previous
                  </button>

                  {/* 当前页码 */}
                  <span className='ml-2 mr-2 text-lg font-bold text-gray-900' style={{ fontSize: '14px' }}>
                    {currentPage}
                  </span>

                  {/* Next 按钮 */}
                  <button
                    onClick={() => setCurrentPage(Math.min(5, currentPage + 1))}
                    disabled={currentPage === 5}
                    className='flex items-center text-gray-400 hover:text-gray-600 disabled:cursor-not-allowed disabled:opacity-50'
                  >
                    Next
                    <svg className='ml-1 h-4 w-4' fill='none' stroke='currentColor' viewBox='0 0 24 24'>
                      <path strokeLinecap='round' strokeLinejoin='round' strokeWidth={2} d='M9 5l7 7-7 7' />
                    </svg>
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </CrateInfoLayout>
    </>
  )
}

// 添加 getProviders 方法以适配新的项目结构
DependenciesGraphPage.getProviders = (page: any, pageProps: any) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default DependenciesGraphPage

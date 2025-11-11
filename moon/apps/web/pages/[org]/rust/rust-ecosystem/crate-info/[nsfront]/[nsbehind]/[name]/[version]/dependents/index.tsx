'use client'

import React, { useEffect, useState } from 'react'
import Head from 'next/head'
import { useParams } from 'next/navigation'
import { useRouter } from 'next/router'

import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'

// import { MagnifyingGlassIcon } from '@heroicons/react/24/outline';
import CrateInfoLayout from '../layout'

interface DependentData {
  crate_name: string
  version: string
  relation: string
}

interface DependentInfo {
  direct_count: number
  indirect_count: number
  data: DependentData[]
}

interface Dependent extends DependentData {
  id: string
  expanded?: boolean
  description?: string
  published?: string
}

const DependentsPage = () => {
  const params = useParams()
  const router = useRouter()
  const [dependents, setDependents] = useState<Dependent[]>([])
  const [currentPage, setCurrentPage] = useState(1)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [stats, setStats] = useState({ direct: 0, indirect: 0 })
  const searchTerm = ''

  // 从URL参数中获取crate信息
  const crateName = (params?.name as string) || 'tokio'
  const version = (params?.version as string) || '1.2.01'
  const nsfront = (params?.nsfront as string) || (router.query.org as string)
  const nsbehind = (params?.nsbehind as string) || 'rust/rust-ecosystem/crate-info'

  // 从 API 获取 dependents 数据
  useEffect(() => {
    const fetchDependents = async () => {
      if (!crateName || !version || !nsfront || !nsbehind) return

      try {
        setLoading(true)
        setError(null)

        const apiBaseUrl = process.env.NEXT_PUBLIC_CRATES_PRO_URL
        const response = await fetch(
          `${apiBaseUrl}/api/crates/${nsfront}/${nsbehind}/${crateName}/${version}/dependents`
        )

        if (!response.ok) {
          throw new Error('Failed to fetch dependents')
        }

        const data: DependentInfo = await response.json()

        // 转换 API 数据为前端需要的格式
        const transformedDependents: Dependent[] = data.data.map((dep, index) => ({
          id: `${dep.crate_name}-${dep.version}-${index}`,
          crate_name: dep.crate_name,
          version: dep.version,
          relation: dep.relation as 'Direct' | 'Indirect',
          expanded: false,
          description: `Dependent package: ${dep.crate_name}`,
          published: 'Unknown'
        }))

        setDependents(transformedDependents)
        setStats({
          direct: data.direct_count,
          indirect: data.indirect_count
        })
      } catch (err) {
        setError('Failed to load dependents')
      } finally {
        setLoading(false)
      }
    }

    fetchDependents()
  }, [crateName, version, nsfront, nsbehind])

  const filteredDependents = dependents.filter(
    (dep) =>
      dep.crate_name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      dep.version.toLowerCase().includes(searchTerm.toLowerCase())
  )

  return (
    <>
      <Head>
        <title>Dependents - {crateName}</title>
      </Head>
      <CrateInfoLayout>
        {/* 主要内容区域 */}
        <div className='flex justify-center'>
          <div className='w-[1370px] px-8 py-4'>
            {/* 统一的白色面板 */}
            <div className='rounded-lg border border-gray-200 bg-white shadow-sm'>
              {/* 数据统计显示 - 在面板内部 */}
              <div className='border-b border-gray-200 p-4'>
                <div className='flex items-center'>
                  <div className='flex flex-col space-y-2' style={{ marginLeft: '8px' }}>
                    <span
                      style={{
                        display: '-webkit-box',
                        WebkitBoxOrient: 'vertical',
                        WebkitLineClamp: 1,
                        overflow: 'hidden',
                        color: '#1c2024',
                        textOverflow: 'ellipsis',
                        fontFamily: '"HarmonyOS Sans SC"',
                        fontSize: '14px',
                        fontStyle: 'normal',
                        fontWeight: '400',
                        lineHeight: '20px',
                        letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                      }}
                    >
                      Direct
                    </span>
                    <span
                      style={{
                        display: '-webkit-box',
                        WebkitBoxOrient: 'vertical',
                        WebkitLineClamp: 1,
                        overflow: 'hidden',
                        color: '#1c2024',
                        textOverflow: 'ellipsis',
                        fontFamily: '"HarmonyOS Sans SC"',
                        fontSize: '14px',
                        fontStyle: 'normal',
                        fontWeight: '400',
                        lineHeight: '20px',
                        letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                      }}
                    >
                      Indirect
                    </span>
                  </div>

                  <div className='ml-8 flex flex-col items-end space-y-2' style={{ marginLeft: '600px' }}>
                    <span
                      style={{
                        display: '-webkit-box',
                        WebkitBoxOrient: 'vertical',
                        WebkitLineClamp: 1,
                        overflow: 'hidden',
                        color: '#3e63dd',
                        textOverflow: 'ellipsis',
                        fontFamily: '"HarmonyOS Sans SC"',
                        fontSize: '14px',
                        fontStyle: 'normal',
                        fontWeight: '400',
                        lineHeight: '20px',
                        letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                      }}
                    >
                      {stats.direct}
                    </span>
                    <span
                      style={{
                        display: '-webkit-box',
                        WebkitBoxOrient: 'vertical',
                        WebkitLineClamp: 1,
                        overflow: 'hidden',
                        color: '#3e63dd',
                        textOverflow: 'ellipsis',
                        fontFamily: '"HarmonyOS Sans SC"',
                        fontSize: '14px',
                        fontStyle: 'normal',
                        fontWeight: '400',
                        lineHeight: '20px',
                        letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                      }}
                    >
                      {stats.indirect}
                    </span>
                  </div>

                  {/* 进度条 */}
                  <div className='ml-4 flex flex-col space-y-2' style={{ width: '596px' }}>
                    <div
                      className='h-2 overflow-hidden rounded-lg'
                      style={{ marginTop: '-2px', backgroundColor: 'rgb(238,238,241)' }}
                    >
                      <div
                        className='h-full rounded-lg'
                        style={{
                          width:
                            stats.direct + stats.indirect > 0
                              ? `${(stats.direct / (stats.direct + stats.indirect)) * 100}%`
                              : '0%',
                          backgroundColor: 'rgb(61,98,220)'
                        }}
                      />
                    </div>
                    <div
                      className='h-2 overflow-hidden rounded-lg'
                      style={{ marginTop: '18px', backgroundColor: 'rgb(238,238,241)' }}
                    >
                      <div
                        className='h-full rounded-lg'
                        style={{
                          width:
                            stats.direct + stats.indirect > 0
                              ? `${(stats.indirect / (stats.direct + stats.indirect)) * 100}%`
                              : '0%',
                          backgroundColor: 'rgb(61,98,220)'
                        }}
                      />
                    </div>
                  </div>
                </div>
              </div>

              {/* 加载状态 */}
              {loading && (
                <div className='flex items-center justify-center py-8'>
                  <div className='text-gray-500'>Loading dependents...</div>
                </div>
              )}

              {/* 错误状态 */}
              {error && (
                <div className='flex items-center justify-center py-8'>
                  <div className='text-red-500'>{error}</div>
                </div>
              )}

              {/* 表格 - 在面板内部 */}
              {!loading && !error && (
                <div className='overflow-x-auto'>
                  <table className='min-w-full divide-y divide-gray-200'>
                    <thead style={{ background: '#ffffff00' }}>
                      <tr>
                        <th className='px-6 py-3 text-left' style={{ marginRight: '20px', marginLeft: '-12px' }}>
                          <span
                            style={{
                              display: '-webkit-box',
                              WebkitBoxOrient: 'vertical',
                              WebkitLineClamp: 1,
                              overflow: 'hidden',
                              color: '#1c2024',
                              textOverflow: 'ellipsis',
                              fontFamily: '"HarmonyOS Sans SC"',
                              fontSize: '14px',
                              fontStyle: 'normal',
                              fontWeight: '400',
                              lineHeight: '20px',
                              letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                            }}
                          >
                            Package
                          </span>
                        </th>
                        <th className='px-6 py-3 text-right' style={{ paddingLeft: '300px' }}>
                          <span
                            style={{
                              display: '-webkit-box',
                              WebkitBoxOrient: 'vertical',
                              WebkitLineClamp: 1,
                              overflow: 'hidden',
                              color: '#1c2024',
                              textOverflow: 'ellipsis',
                              fontFamily: '"HarmonyOS Sans SC"',
                              fontSize: '14px',
                              fontStyle: 'normal',
                              fontWeight: '400',
                              lineHeight: '20px',
                              letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                            }}
                          >
                            Version
                          </span>
                        </th>
                        <th className='px-6 py-3 text-right'>
                          <span
                            style={{
                              display: '-webkit-box',
                              WebkitBoxOrient: 'vertical',
                              WebkitLineClamp: 1,
                              overflow: 'hidden',
                              color: '#1c2024',
                              textOverflow: 'ellipsis',
                              fontFamily: '"HarmonyOS Sans SC"',
                              fontSize: '14px',
                              fontStyle: 'normal',
                              fontWeight: '400',
                              lineHeight: '20px',
                              letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                            }}
                          >
                            Relation
                          </span>
                        </th>
                      </tr>
                    </thead>
                    <tbody className='divide-y divide-gray-200 bg-white'>
                      {filteredDependents.map((dependent) => (
                        <React.Fragment key={dependent.id}>
                          <tr className='hover:bg-gray-50'>
                            <td className='whitespace-nowrap px-6 py-4' style={{ paddingLeft: '24px' }}>
                              <div
                                className='cursor-pointer hover:underline'
                                style={{
                                  display: '-webkit-box',
                                  WebkitBoxOrient: 'vertical',
                                  WebkitLineClamp: 1,
                                  overflow: 'hidden',
                                  color: '#002bb7c4',
                                  textOverflow: 'ellipsis',
                                  fontFamily: '"HarmonyOS Sans SC"',
                                  fontSize: '14px',
                                  fontStyle: 'normal',
                                  fontWeight: 400,
                                  lineHeight: '20px',
                                  letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                }}
                              >
                                {dependent.crate_name}
                              </div>
                            </td>
                            <td className='whitespace-nowrap px-6 py-4 text-right' style={{ paddingLeft: '300px' }}>
                              <span
                                className='cursor-pointer hover:underline'
                                style={{
                                  display: '-webkit-box',
                                  WebkitBoxOrient: 'vertical',
                                  WebkitLineClamp: 1,
                                  overflow: 'hidden',
                                  color: '#1c2024',
                                  textOverflow: 'ellipsis',
                                  fontFamily: '"SF Pro"',
                                  fontSize: '14px',
                                  fontStyle: 'normal',
                                  fontWeight: 400,
                                  lineHeight: '20px',
                                  letterSpacing: 'var(--Typography-Letter-spacing-2, 0)',
                                  marginRight: '16px'
                                }}
                              >
                                {dependent.version}
                              </span>
                            </td>
                            <td className='whitespace-nowrap px-6 py-4 text-right'>
                              <span
                                className='cursor-pointer hover:underline'
                                style={{
                                  display: '-webkit-box',
                                  WebkitBoxOrient: 'vertical',
                                  WebkitLineClamp: 1,
                                  overflow: 'hidden',
                                  color: '#002bb7c4',
                                  textOverflow: 'ellipsis',
                                  fontFamily: '"HarmonyOS Sans SC"',
                                  fontSize: '14px',
                                  fontStyle: 'normal',
                                  fontWeight: 400,
                                  lineHeight: '20px',
                                  letterSpacing: 'var(--Typography-Letter-spacing-2, 0)',
                                  marginRight: '14px'
                                }}
                              >
                                {dependent.relation}
                              </span>
                            </td>
                          </tr>
                          {dependent.expanded && dependent.description && (
                            <tr className='bg-gray-50'>
                              <td colSpan={3} className='px-6 py-4'>
                                <div className='space-y-2'>
                                  <div className='flex items-center space-x-4'>
                                    <span className='cursor-pointer text-sm text-blue-600 hover:text-blue-800'>
                                      Version: {dependent.version}
                                    </span>
                                    <span className='text-sm text-gray-500'>Published: {dependent.published}</span>
                                  </div>
                                  <p className='text-sm text-gray-700'>{dependent.description}</p>
                                </div>
                              </td>
                            </tr>
                          )}
                        </React.Fragment>
                      ))}
                    </tbody>
                  </table>
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
DependentsPage.getProviders = (page: any, pageProps: any) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default DependentsPage

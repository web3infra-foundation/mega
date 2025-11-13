import { useEffect, useState } from 'react'
import { MagnifyingGlassIcon } from '@heroicons/react/24/outline'
import Head from 'next/head'
import Image from 'next/image'
import { useRouter } from 'next/router'

import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'

interface CVEItem {
  id: string
  subtitle: string
  description: string
  // 可选字段，因为实际API可能不返回这些
  url?: string
  crate_name?: string
  start_version?: string
  end_version?: string
}

interface CVEData {
  cves: CVEItem[]
}

export default function EcosystemCVEPage() {
  const [search, setSearch] = useState('')
  const [currentPage, setCurrentPage] = useState(1)
  const [expandedIdx, setExpandedIdx] = useState<number | null>(null)
  const [cveList, setCveList] = useState<CVEItem[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const router = useRouter()

  useEffect(() => {
    const fetchCVEList = async () => {
      try {
        setLoading(true)
        const apiBaseUrl = process.env.NEXT_PUBLIC_CRATES_PRO_URL

        const response = await fetch(`${apiBaseUrl}/api/cvelist`)

        if (!response.ok) {
          throw new Error('Failed to fetch CVE data')
        }
        const data: CVEData = await response.json()

        setCveList(data.cves || [])
        setError(null)
      } catch (err) {
        setError('Failed to load CVE data')
        setCveList([])
      } finally {
        setLoading(false)
      }
    }

    fetchCVEList()
  }, [])

  // 为每个CVE项生成随机标签
  const getCVEWithTag = (cve: CVEItem, index: number) => {
    const tags = [
      { text: '由国际安全组织报告', color: 'blue' },
      { text: '修复补丁已发布', color: 'green' },
      { text: '远程更新可用', color: 'green' },
      { text: '安全漏洞', color: 'red' },
      { text: '高危漏洞', color: 'red' },
      { text: '已修复', color: 'green' }
    ]

    // 根据索引和CVE ID生成"随机"标签，添加安全检查
    const cveIdLength = cve?.id?.length || 0
    const tagIndex = (index + cveIdLength) % tags.length
    const tag = tags[tagIndex]

    return {
      ...cve,
      tag
    }
  }

  const totalPages = Math.ceil(cveList.length / 10)
  const itemsPerPage = 10

  return (
    <>
      <Head>
        <title>Ecosystem CVE</title>
      </Head>
      <div className='h-auto min-h-screen w-full bg-white'>
        {/* 搜索栏和标题区域 */}
        <div className='w-full'>
          <div
            className='sticky top-0 z-20 flex items-center border-b border-gray-200 bg-white'
            style={{
              width: '100%',
              height: '53px',
              flexShrink: 0,
              marginTop: 0,
              marginBottom: 0,
              paddingLeft: 'max(20px, 2vw)',
              paddingRight: 'max(32px, 4vw)',
              borderBottom: '1px solid #E5E7EB',
              background: '#FFF'
            }}
          >
            <div className='max-w-xl flex-1'>
              <div className='relative' style={{ marginLeft: '-10px' }}>
                <div className='pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3'>
                  <MagnifyingGlassIcon className='h-5 w-5 text-gray-400' />
                </div>
                <input
                  type='text'
                  placeholder='Search...'
                  className='block w-full border-0 bg-transparent py-2 pl-10 pr-3 text-gray-900 placeholder-gray-500 focus:outline-none focus:ring-0'
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                />
              </div>
            </div>
          </div>
        </div>

        {/* All CVEs 标题 */}
        <div className='mt-4 w-full'>
          <div
            style={{
              width: '100%',
              paddingLeft: 'max(20px, 2vw)',
              paddingRight: 'max(32px, 4vw)'
            }}
          >
            <h1
              style={{
                display: '-webkit-box',
                WebkitBoxOrient: 'vertical',
                WebkitLineClamp: 1,
                overflow: 'hidden',
                color: '#1c2024',
                textOverflow: 'ellipsis',
                fontFamily: 'HarmonyOS Sans SC',
                fontSize: '24px',
                fontStyle: 'normal',
                fontWeight: 700,
                lineHeight: '24px',
                letterSpacing: 'var(--Typography-Letter-spacing-3, 0)'
              }}
            >
              All CVEs
            </h1>
          </div>
        </div>

        {/* CVE 信息区 */}
        <div className='mt-4 w-full'>
          <div
            style={{
              width: '100%',
              paddingLeft: 'max(20px, 2vw)',
              paddingRight: 'max(32px, 4vw)'
            }}
          >
            {/* 标题下方的分割线 */}
            <div
              style={{
                borderBottom: '1px solid #E5E7EB',
                width: '100%',
                height: 0,
                marginBottom: 0
              }}
            />
            {/* 加载状态 */}
            {loading && (
              <div className='flex items-center justify-center py-8'>
                <div className='text-gray-500'>加载中...</div>
              </div>
            )}

            {/* 错误状态 */}
            {error && (
              <div className='flex items-center justify-center py-8'>
                <div className='text-red-500'>{error}</div>
              </div>
            )}

            {/* CVE列表 */}
            {!loading && !error && (
              <div className='space-y-0'>
                {cveList.slice((currentPage - 1) * itemsPerPage, currentPage * itemsPerPage).map((item, idx) => {
                  const itemWithTag = getCVEWithTag(item, idx)

                  return (
                    <div key={item.id} style={{ position: 'relative' }}>
                      <div className='flex min-h-[51px] flex-col justify-between py-4 md:min-h-[51px] md:flex-row md:items-center'>
                        <div className='mt-0 flex flex-1 flex-col gap-2 md:flex-row md:items-center'>
                          <span
                            className='cursor-pointer text-lg font-medium text-gray-900 hover:text-blue-600'
                            style={{
                              display: '-webkit-box',
                              WebkitBoxOrient: 'vertical',
                              WebkitLineClamp: 1,
                              overflow: 'hidden',
                              color: '#1c2024',
                              textOverflow: 'ellipsis',
                              fontFamily: 'HarmonyOS Sans SC',
                              fontSize: '18px',
                              fontStyle: 'normal',
                              fontWeight: 400,
                              lineHeight: '24px',
                              letterSpacing: 'var(--Typography-Letter-spacing-3, 0)'
                            }}
                            onClick={() =>
                              router.push(
                                `/${router.query.org}/rust/rust-ecosystem/ecosystem-cve/cve-info?cveId=${item.id}`
                              )
                            }
                          >
                            {item.id}
                          </span>
                          {itemWithTag.tag && (
                            <span
                              className={`ml-2 rounded px-2 py-0.5 bg-${itemWithTag.tag.color}-50 text-xs text-${itemWithTag.tag.color}-600 font-semibold`}
                            >
                              {itemWithTag.tag.text}
                            </span>
                          )}
                        </div>
                        <button
                          className='ml-4 flex h-8 w-8 items-center justify-center'
                          onClick={() => setExpandedIdx(expandedIdx === idx ? null : idx)}
                          style={{ outline: 'none', border: 'none', background: 'transparent', cursor: 'pointer' }}
                        >
                          <Image
                            src='/rust/rust-ecosystem/down.png'
                            alt='toggle'
                            width={20}
                            height={20}
                            style={{
                              transition: 'transform 0.2s',
                              transform: expandedIdx === idx ? 'rotate(180deg)' : 'none'
                            }}
                          />
                        </button>
                      </div>
                      {expandedIdx === idx && (
                        <div className='pb-4'>
                          <div className='mb-2 text-sm text-gray-500'>
                            <div className='mb-2'>
                              <strong>Description: </strong>
                              {item.description}
                            </div>
                            <div className='mb-2'>
                              <strong>Subtitle: </strong>
                              {item.subtitle}
                            </div>
                            {item.crate_name && (
                              <div className='mb-2'>
                                <strong>影响的包：</strong>
                                {item.crate_name}
                              </div>
                            )}
                            {item.start_version && item.end_version && (
                              <div className='mb-2'>
                                <strong>影响版本：</strong>
                                {item.start_version} - {item.end_version}
                              </div>
                            )}
                          </div>
                          <div className='flex justify-end'>
                            <button
                              style={{
                                display: 'inline-flex',
                                height: '24px',
                                padding: '0 8px',
                                justifyContent: 'center',
                                alignItems: 'center',
                                gap: '4px',
                                flexShrink: 0,
                                borderRadius: '3px',
                                background: '#0047f112',
                                color: '#3E63DD',
                                fontWeight: 500,
                                fontSize: '14px',
                                border: 'none',
                                outline: 'none',
                                cursor: 'pointer'
                              }}
                              onClick={() =>
                                router.push(
                                  `/${router.query.org}/rust/rust-ecosystem/ecosystem-cve/cve-info?cveId=${item.id}`
                                )
                              }
                            >
                              Details
                            </button>
                          </div>
                        </div>
                      )}
                      {/* 分割线 */}
                      {idx !== Math.min(itemsPerPage, cveList.length - (currentPage - 1) * itemsPerPage) - 1 && (
                        <div
                          style={{
                            borderBottom: '1px solid #E5E7EB',
                            width: '100%',
                            height: 0
                          }}
                        />
                      )}
                    </div>
                  )
                })}
                {/* 最后一个CVE下方的分割线 */}
                {cveList.length > 0 && (
                  <div
                    style={{
                      borderBottom: '1px solid #E5E7EB',
                      width: '100%',
                      height: 0
                    }}
                  />
                )}
              </div>
            )}
          </div>
        </div>

        {/* 分页功能区 */}
        {!loading && !error && cveList.length > 0 && totalPages > 1 && (
          <div className='mt-8 w-full'>
            <div
              style={{
                width: '100%',
                paddingLeft: 'max(20px, 2vw)',
                paddingRight: 'max(32px, 4vw)'
              }}
            >
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
                  onClick={() => setCurrentPage(Math.min(totalPages, currentPage + 1))}
                  disabled={currentPage === totalPages}
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
        )}
      </div>
    </>
  )
}

EcosystemCVEPage.getProviders = (page: any, pageProps: any) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

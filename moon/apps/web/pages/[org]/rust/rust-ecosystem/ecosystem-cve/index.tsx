import Head from 'next/head'
import Image from 'next/image'
import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { useState } from 'react'
import { MagnifyingGlassIcon } from '@heroicons/react/24/outline'
import { useRouter } from 'next/router'

export default function EcosystemCVEPage() {
  const [search, setSearch] = useState('')
  const [currentPage, setCurrentPage] = useState(1)
  const [expandedIdx, setExpandedIdx] = useState<number | null>(null)
  const router = useRouter()

  const cveList = [
    {
      id: 100,
      title: 'CVE-2023-12345:示例漏洞描述',
      tag: { text: '修复补丁已发布', color: 'green' },
      detail: '该漏洞的修复补丁已发布，建议尽快升级。',
    },
    {
      id: 101,
      title: 'CVE-2023-12345:示例漏洞描述',
      tag: { text: '远程更新可用', color: 'green' },
      detail: '这是CVE-2023-12345的详细内容。此漏洞可能导致用户数据被曝光，并影响应用程序的性能',
    },
    {
      id: 102,
      title: 'CVE-2023-12345:示例漏洞描述',
      tag: null,
      detail: '暂无更多详情。',
    },
    {
      id: 104,
      title: 'CVE-2023-12345:示例漏洞描述',
      tag: { text: '修复补丁已发布', color: 'green' },
      detail: '该漏洞的修复补丁已发布，建议尽快升级。',
    },
    {
      id: 105,
      title: 'CVE-2023-12345:示例漏洞描述',
      tag: null,
      detail: '暂无更多详情。',
    },
    {
      id: 107,
      title: 'CVE-2023-12345:示例漏洞描述',
      tag: { text: '由国际安全组织报告', color: 'blue' },
      detail: '该漏洞由国际安全组织披露，建议关注官方通告。',
    },
    {
      id: 108,
      title: 'CVE-2023-12345:示例漏洞描述',
      tag: { text: '由国际安全组织报告', color: 'blue' },
      detail: '该漏洞由国际安全组织披露，建议关注官方通告。',
    },
    {
      id: 109,
      title: 'CVE-2023-12345:示例漏洞描述',
      tag: { text: '由国际安全组织报告', color: 'blue' },
      detail: '该漏洞由国际安全组织披露，建议关注官方通告。',
    },
    {
      id: 110,
      title: 'CVE-2023-12345:示例漏洞描述',
      tag: { text: '由国际安全组织报告', color: 'blue' },
      detail: '该漏洞由国际安全组织披露，建议关注官方通告。',
    },
    {
      id: 111,
      title: 'CVE-2023-12345:示例漏洞描述',
      tag: { text: '由国际安全组织报告', color: 'blue' },
      detail: '该漏洞由国际安全组织披露，建议关注官方通告。',
    },
    
  ]

  const totalPages = 5
  // const itemsPerPage = 10

  return (
    <>
      <Head>
        <title>Ecosystem CVE</title>
      </Head>
      <div className="min-h-screen h-auto w-full bg-white">
        {/* 搜索栏和标题区域 */}
        <div className="w-full flex justify-center">
          <div
            className="flex items-center border-b border-gray-200 bg-white sticky top-0 z-20"
            style={{
              width: '1680px',
              height: '53px',
              flexShrink: 0,
              marginTop: 0,
              marginBottom: 0,
              paddingLeft: 32,
              paddingRight: 32,
              borderBottom: '1px solid #E5E7EB',
              background: '#FFF',
            }}
          >
            <div className="flex-1 max-w-xl">
              <div className="relative ml-10">
                <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                  <MagnifyingGlassIcon className="h-5 w-5 text-gray-400" />
                </div>
                <input
                  type="text"
                  placeholder="Search..."
                  className="block w-full pl-10 pr-3 py-2 border-0 focus:ring-0 focus:outline-none bg-transparent text-gray-900 placeholder-gray-500"
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                />
              </div>
            </div>
          </div>
        </div>

        {/* All CVEs 标题 */}
        <div className="w-full flex justify-center mt-4">
          <div style={{ width: '1680px', paddingLeft: 32 }}>
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
                letterSpacing: 'var(--Typography-Letter-spacing-3, 0)',
              }}
            >
              All CVEs
            </h1>
          </div>
        </div>

        {/* CVE 信息区 */}
        <div className="w-full flex justify-center mt-4">
          <div style={{ width: '1680px', paddingLeft: 32, paddingRight: 32 }}>
            {/* 标题下方的分割线 */}
            <div
              style={{
                borderBottom: '1px solid #E5E7EB',
                width: '100%',
                height: 0,
                marginBottom: 0,
              }}
            />
            {/* CVE列表 */}
            <div className="space-y-0">
              {cveList.map((item, idx) => (
                <div key={item.id} style={{ position: 'relative' }}>
                  <div
                    className="flex flex-col md:flex-row md:items-center  justify-between py-4 min-h-[51px] md:min-h-[51px] md:items-center"
                  >
                    <div className="flex flex-col md:flex-row md:items-center gap-2 mt-0 flex-1">
                      <span 
                        className="font-medium text-lg text-gray-900 cursor-pointer hover:text-blue-600"
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
                          letterSpacing: 'var(--Typography-Letter-spacing-3, 0)',
                        }}
                        onClick={() => router.push(`/${router.query.org}/rust/rust-ecosystem/ecosystem-cve/cve-info/${item.id}`)}
                      >
                        {item.title}
                      </span>
                      {item.tag && (
                        <span className={`ml-2 px-2 py-0.5 rounded bg-${item.tag.color}-50 text-xs text-${item.tag.color}-600 font-semibold`}>
                          {item.tag.text}
                        </span>
                      )}
                    </div>
                    <button
                      className="ml-4 flex items-center justify-center w-8 h-8"
                      onClick={() => setExpandedIdx(expandedIdx === idx ? null : idx)}
                      style={{ outline: 'none', border: 'none', background: 'transparent', cursor: 'pointer' }}
                    >
                      <Image
                        src="/rust/rust-ecosystem/down.png"
                        alt="toggle"
                        width={20}
                        height={20}
                        style={{transition: 'transform 0.2s', transform: expandedIdx === idx ? 'rotate(180deg)' : 'none' }}
                      />
                    </button>
                  </div>
                  {expandedIdx === idx && (
                    <div className="pb-4">
                      <div className="text-gray-500 text-sm mb-2">
                        {item.detail}
                      </div>
                      <div className="flex justify-end">
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
                            cursor: 'pointer',
                          }}
                        >
                          Details
                        </button>
                      </div>
                    </div>
                  )}
                  {/* 分割线 */}
                  {idx !== cveList.length - 1 && (
                    <div
                      style={{
                        borderBottom: '1px solid #E5E7EB',
                        width: '100%',
                        height: 0,
                      }}
                    />
                  )}
                </div>
              ))}
              {/* 最后一个CVE下方的分割线 */}
              <div
                style={{
                  borderBottom: '1px solid #E5E7EB',
                  width: '100%',
                  height: 0,
                }}
              />
            </div>
          </div>
        </div>

        {/* 分页功能区 */}
        <div className="w-full flex justify-center mt-8 ">
          <div style={{ width: '1680px', paddingLeft: 32, paddingRight: 32 }}>
            <div className="flex justify-center items-center gap-6" style={{ marginLeft: '-100px' }}>
              {/* Previous 按钮 */}
              <button
                onClick={() => setCurrentPage(Math.max(1, currentPage - 1))}
                disabled={currentPage === 1}
                className="flex items-center text-gray-400 hover:text-gray-600 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
                </svg>
                Previous
              </button>

              {/* 当前页码 */}
              <span className="text-lg font-bold text-gray-900 ml-2 mr-2" style={{ fontSize: '14px' }}>{currentPage}</span>

              {/* Next 按钮 */}
              <button
                onClick={() => setCurrentPage(Math.min(totalPages, currentPage + 1))}
                disabled={currentPage === totalPages}
                className="flex items-center text-gray-400 hover:text-gray-600 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                Next
                <svg className="w-4 h-4 ml-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                </svg>
              </button>
            </div>
          </div>
        </div>
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

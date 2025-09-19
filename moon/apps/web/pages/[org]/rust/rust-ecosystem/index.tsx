import Head from 'next/head'
import Image from 'next/image'
import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { useState, useEffect } from 'react'
import { MagnifyingGlassIcon } from '@heroicons/react/24/outline'
import { useRouter } from 'next/router'

interface CVEItem {
  cve_id: string
  url: string
  description: string
  crate_name: string
  start_version: string
  end_version: string
}

interface CVEData {
  cves: CVEItem[]
}

export default function RustEcosystemPage() {
  const [search, setSearch] = useState('')
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
    
    // 根据索引和CVE ID生成"随机"标签
    const tagIndex = (index + cve.cve_id.length) % tags.length
    const tag = tags[tagIndex]
    
    return {
      ...cve,
      tag
    }
  }

  return (
    <>
      <Head>
        <title>Crate Ecosystem</title>
      </Head>
      <div className="h-screen overflow-auto">
        {/* 顶部搜索区，带背景图 */}
        <div
          className="w-full overflow-hidden mb-6 flex flex-col justify-center items-center relative"
          style={{
            height: '160px',
            minHeight: '160px',
          }}
        >
          {/* 背景图层 */}
          <div
            className="absolute inset-0"
            style={{
              backgroundImage: 'url(/rust/rust-ecosystem/search-bg.png)',
              backgroundSize: 'cover',
              backgroundPosition: 'center',
              backgroundRepeat: 'no-repeat',
              backgroundAttachment: 'fixed',
            }}
          />
          {/* 内容层 */}
          <div className="flex flex-col items-center justify-center w-full h-full backdrop-blur-sm bg-white/70 relative z-10">
         
            <div className="flex justify-center w-full">
              <div className="relative flex items-center w-full max-w-4xl" style={{ padding: '0 8px' }}>
                <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                  <MagnifyingGlassIcon className="h-5 w-5 text-gray-400" />
                </div>
                <input
                  type="text"
                  placeholder="Search the crate..."
                  className="block pl-10 pr-3 py-3 border border-gray-200 rounded-lg focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 bg-white/90 flex-grow"
                  style={{ minWidth: 0 }}
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  onKeyPress={(e) => {
                    if (e.key === 'Enter') {
                      e.preventDefault()
                      if (search.trim()) {
                        router.push(`/${router.query.org}/rust/rust-ecosystem/search?q=${encodeURIComponent(search.trim())}`)
                      }
                    }
                  }}
                />
                <button
                  type="button"
                  className="inline-flex items-center justify-center h-12 px-6 ml-3 gap-3 rounded-lg bg-[#1F2D5C] text-white text-base font-medium whitespace-nowrap flex-shrink-0"
                  style={{ height: '48px', padding: '0 24px', borderRadius: '8px', background: '#1F2D5C' }}
                  onClick={() => {
                    if (search.trim()) {
                      router.push(`/${router.query.org}/rust/rust-ecosystem/search?q=${encodeURIComponent(search.trim())}`)
                    }
                  }}
                >
                  Search Crate
                </button>
              </div>
            </div>
          </div>
        </div>

        {/* 四个组件卡片区 */}
        <div className="w-full flex justify-center gap-6 mt-1 mb-2 flex-wrap">
          {/* crate 卡片 */}
          <div
            style={{ 
              width: 290, 
              height: 188, 
              borderRadius: 12, 
              background: 'linear-gradient(180deg, #F3F1FF 0%, #FFFFFF 100%)',
              backgroundClip: 'padding-box, border-box',
              backgroundOrigin: 'padding-box, border-box',
              backgroundImage: 'linear-gradient(to right, #F3F1FF, #FFFFFF), linear-gradient(135deg, #DCE1FE, #B8C3FF)',
              border: '2px solid transparent',
            }}
            className="flex flex-col items-center justify-center flex-shrink-0 overflow-hidden"
          >
            <Image 
              src="/rust/rust-ecosystem/crate.png" 
              alt="crate" 
              width={120}
              height={120}
               />
            <span
              style={{
                color: '#aa99ec',
                fontFamily: 'HarmonyOS Sans SC',
                fontSize: 36,
                fontStyle: 'normal',
                fontWeight: 700,
                lineHeight: '38px',
                letterSpacing: 0,
              }}
            >
              Crate
            </span>
          </div>
          {/* cve 卡片 */}
          <div
            style={{ 
              width: 290, 
              height: 188, 
              borderRadius: 12, 
              background: 'linear-gradient(180deg, #FFF8E1 0%, #FFFFFF 100%)',
              backgroundClip: 'padding-box, border-box',
              backgroundOrigin: 'padding-box, border-box',
              backgroundImage: 'linear-gradient(to right, #FFF8E1, #FFFFFF), linear-gradient(135deg, #FFF4DB, #FFDE94)',
              border: '2px solid transparent',
            }}
            className="flex flex-col items-center justify-center flex-shrink-0 overflow-hidden cursor-pointer hover:shadow-lg transition-shadow"
            onClick={() => router.push(`/${router.query.org}/rust/rust-ecosystem/ecosystem-cve`)}
          >
            <Image 
              src="/rust/rust-ecosystem/cve.png" 
              alt="cve" 
              width={120}
              height={120}
              />
            <span
              style={{
                color: '#ffc53d',
                fontFamily: 'HarmonyOS Sans SC',
                fontSize: 36,
                fontStyle: 'normal',
                fontWeight: 700,
                lineHeight: '38px',
                letterSpacing: 0,
              }}
            >
              CVE
            </span>
          </div>
          {/* rust 卡片 */}
          <div
            style={{ 
              width: 290, 
              height: 188, 
              borderRadius: 12, 
              background: 'linear-gradient(180deg, #FFEFE7 0%, #FFFFFF 100%)',
              backgroundClip: 'padding-box, border-box',
              backgroundOrigin: 'padding-box, border-box',
              backgroundImage: 'linear-gradient(to right, #FFEFE7, #FFFFFF), linear-gradient(135deg, #FFF0E6, #FFC8A6)',
              border: '2px solid transparent',
            }}
            className="flex flex-col items-center justify-center flex-shrink-0 overflow-hidden"
          >
            <Image 
              src="/rust/rust-ecosystem/rust.png" 
              alt="cve" 
              width={120}
              height={120}
              />
            <span
              style={{
                color: '#ff8d47',
                fontFamily: 'HarmonyOS Sans SC',
                fontSize: 36,
                fontStyle: 'normal',
                fontWeight: 700,
                lineHeight: '38px',
                letterSpacing: 0,
              }}
            >
              Rust
            </span>
          </div>
          {/* code 卡片 */}
          <div
            style={{ 
              width: 290, 
              height: 188, 
              borderRadius: 12, 
              background: 'linear-gradient(180deg, #EAF6FF 0%, #FFFFFF 100%)',
              backgroundClip: 'padding-box, border-box',
              backgroundOrigin: 'padding-box, border-box',
              backgroundImage: 'linear-gradient(to right, #EAF6FF, #FFFFFF), linear-gradient(135deg, #DCF1FE, #B8E4FF)',
              border: '2px solid transparent',
            }}
            className="flex flex-col items-center justify-center flex-shrink-0 overflow-hidden"
          >
            <Image 
              src="/rust/rust-ecosystem/code.png" 
              alt="cve" 
              width={120}
              height={120}
              />
            <span
              style={{
                color: '#76bdff',
                fontFamily: 'HarmonyOS Sans SC',
                fontSize: 36,
                fontStyle: 'normal',
                fontWeight: 700,
                lineHeight: '38px',
                letterSpacing: 0,
              }}
            >
              Code
            </span>
          </div>
        </div>

        {/* CVE 信息区 */}
        <div className="w-full max-w-[1260px] mx-auto px-4 flex mt-8 justify-start mb-32">
          <div style={{ width: 1260 }}>
            {/* 标题区 */}
            <div className="mb-0 ">
              <div
                style={{
                  color: '#00000099',
                  fontFamily: 'HarmonyOS Sans SC',
                  fontSize: 16,
                  fontStyle: 'normal',
                  fontWeight: 500,
                  lineHeight: 'normal',
                  textTransform: 'capitalize',
                }}
                className="mb-0 ml-1"
              >
                CVE Information
              </div>
              <div
                style={{
                  color: '#000000',
                  fontFamily: 'HarmonyOS Sans SC',
                  fontSize: 60,
                  fontStyle: 'normal',
                  fontWeight: 500,
                  lineHeight: 'normal',
                  textTransform: 'capitalize',
                }}
                className="mb-4 "
              >
                CVE信息
              </div>
            </div>
            {/* 信息列表卡片 */}
            <div
              style={{
                width: 1260,
                height: 583,
                flexShrink: 0,
                borderRadius: 12,
                border: '1px solid #EBEBFF',
                background: 'rgb(250,251,251)',
                boxShadow: '0 4px 16px -8px #0000001a, 0 3px 12px -4px #0000001a, 0 2px 3px -2px #0000330f',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
              }}
              className="mx-auto"
            >
              <div
                style={{
                  width: 1228,
                  height: 551,
                  flexShrink: 0,
                  borderRadius: 12,
                  background: '#FFF',
                  backdropFilter: 'blur(4.51px)',
                  boxShadow: '0 1px 4px 0 rgba(0,0,0,0.04)',
                  padding: 32,
                  display: 'flex',
                  flexDirection: 'column',
                  justifyContent: 'space-between',
                }}
              >
                {/* CVE列表 */}
                <div
                  style={{ paddingTop: 0 }} // 原来是32，改为16让整体往上移
                  className="space-y-0 flex-1 overflow-auto"
                >
                  {/* 加载状态 */}
                  {loading && (
                    <div className="flex justify-center items-center py-8">
                      <div className="text-gray-500">加载中...</div>
                    </div>
                  )}
                  
                  {/* 错误状态 */}
                  {error && (
                    <div className="flex justify-center items-center py-8">
                      <div className="text-red-500">{error}</div>
                    </div>
                  )}
                  
                  {/* CVE列表 */}
                  {!loading && !error && cveList.slice(0, 8).map((item, idx) => {
                    const itemWithTag = getCVEWithTag(item, idx)
                    
                    return (
                      <div key={item.cve_id} style={{ position: 'relative' }}>
                        <div
                          className="flex flex-col md:flex-row md:items-center justify-between pb-4 min-h-[51px] md:min-h-[51px] md:items-center px-2" // 原px-8，改为px-2让内容更宽
                        >
                          <div className="flex flex-col md:flex-row md:items-center gap-2 flex-1">
                            <span 
                              className="font-medium text-lg text-gray-900 cursor-pointer hover:text-blue-600"
                              onClick={() => router.push(`/${router.query.org}/rust/rust-ecosystem/ecosystem-cve/cve-info/${item.cve_id}`)}
                            >
                              {item.cve_id}
                            </span>
                            {itemWithTag.tag && (
                              <span className={`ml-2 px-2 py-0.5 rounded bg-${itemWithTag.tag.color}-50 text-xs text-${itemWithTag.tag.color}-600 font-semibold`}>
                                {itemWithTag.tag.text}
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
                          <div className="px-2 pb-4"> {/* 这里也同步缩小padding */}
                            <div className="text-gray-500 text-sm mb-2">
                              <div className="mb-2">
                                <strong>描述：</strong>{item.description}
                              </div>
                              <div className="mb-2">
                                <strong>影响的包：</strong>{item.crate_name}
                              </div>
                              <div className="mb-2">
                                <strong>影响版本：</strong>{item.start_version} - {item.end_version}
                              </div>
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
                                onClick={() => router.push(`/${router.query.org}/rust/rust-ecosystem/ecosystem-cve/cve-info/${item.cve_id}`)}
                              >
                                Details
                              </button>
                            </div>
                          </div>
                        )}
                        {/* 分割线，放到 .px-8 外面 */}
                        <div
                          style={{
                            position: 'absolute',
                            left: 0,
                            right: 0,
                            bottom: 0,
                            borderBottom: '1px solid #E5E7EB',
                            width: '100%',
                            height: 0,
                          }}
                        />
                      </div>
                    )
                  })}
                </div>
                {/* More 按钮 */}
                <div className="flex justify-end mt-6">
                  <button
                    className="inline-flex items-center justify-center text-base text-white"
                    style={{
                      height: 40,
                      padding: '0 16px',
                      gap: 12,
                      flexShrink: 0,
                      borderRadius: 6,
                      background: '#3E63DD',
                      fontWeight: 500,
                    }}
                    onClick={() => router.push(`/${router.query.org}/rust/rust-ecosystem/ecosystem-cve`)}
                  >
                    More
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </>
  )
}

RustEcosystemPage.getProviders = (page: any, pageProps: any) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

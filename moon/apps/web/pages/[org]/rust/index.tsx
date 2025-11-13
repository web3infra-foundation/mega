import Head from 'next/head'
import { useRouter } from 'next/router'
// import Link from 'next/link'
import { Link } from '@gitmono/ui/Link'
// import { ChatBubblePlusIcon } from '@gitmono/ui/Icons'

import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { PageWithLayout } from '@/utils/types'
import { IndexSearchInput } from '@/components/IndexPages/components'
import { useState, useCallback, useEffect } from 'react'
import Image from 'next/image'

const Logo = () => (
  <div className='flex items-center'>
    <Image
      src='/rust/logo.png'
      alt='Rust Logo'
      width={24}
      height={24}
      className='relative mr-2'
      style={{ opacity: 0.8, top: '-2px' }}
    />
  </div>
)

const CratesproPage: PageWithLayout<any> = () => {
  const router = useRouter()
  const [query, setQuery] = useState('')
  const isSearchLoading = false // 如有异步搜索可改为实际loading

  interface LatestCve {
    id: string
    subtitle: string
    description: string
    issued: string
  }
  const [latestCves, setLatestCves] = useState<LatestCve[]>([])

  const [cveLoading, setCveLoading] = useState<boolean>(false)
  
  const [cveError, setCveError] = useState<string | null>(null)

  // 处理搜索功能
  const handleSearch = useCallback(() => {
    if (query.trim()) {
      const org = router.query.org || 'org'

      router.push(`/${org}/rust/rust-ecosystem/search?q=${encodeURIComponent(query.trim())}`)
    }
  }, [query, router])

  // 在捕获阶段处理键盘事件，这样可以在 IndexSearchInput 阻止之前捕获
  const handleKeyDownCapture = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleSearch()
    }
  }

  const handleNavClick = (navKey: string) => {
    const org = router.query.org ? `/${router.query.org}` : ''

    switch (navKey) {
      case 'crates':
        router.push(`${org}/rust/rust-ecosystem`)
        break
      case 'cves':
        router.push(`${org}/rust/rust-ecosystem/ecosystem-cve`)
        break
      default:
        break
    }
  }

  // 拉取最新 CVEs
  useEffect(() => {

    const fetchLatest = async () => {
      try {
        setCveLoading(true)
        setCveError(null)

        const apiBaseUrl = process.env.NEXT_PUBLIC_CRATES_PRO_URL

        const res = await fetch(`${apiBaseUrl}/api/latestcves`)

        if (!res.ok) throw new Error('failed to load latest cves')
        const json = await res.json()
        // 兼容 { cves: LatestCve[] } 或直接数组
        const list: LatestCve[] = Array.isArray(json) ? json : (json.cves ?? [])

        setLatestCves(list)
      } catch (e) {
        setCveError('Failed to load latest CVEs')
        // fallback: 空数组
        setLatestCves([])
      } finally {
        setCveLoading(false)
      }
    }
    
    fetchLatest()
  }, [])

  return (
    <>
      <Head>
        <title>Rust Ecosystem</title>
      </Head>
      <div className='flex min-h-screen w-full flex-col bg-white'>
        {/* 顶部导航栏 */}
        <div className='flex w-full flex-col'>
          <div className='flex w-full items-center px-8 pb-4 pt-8'>
            {/* 左侧：Logo + 导航按钮 */}
            <div className='flex items-center gap-8'>
              <Logo />

              {/* 导航按钮 */}
              <div className='flex items-center gap-8'>
                <button
                  onClick={() => handleNavClick('crates')}
                  className='font-medium text-gray-700 transition-colors hover:text-blue-600'
                >
                  Crates
                </button>
                <button
                  onClick={() => handleNavClick('cves')}
                  className='font-medium text-gray-700 transition-colors hover:text-blue-600'
                >
                  CVEs
                </button>
              </div>
            </div>

            {/* 中间：搜索栏 */}
            <div className="flex-1 flex justify-end ml-60">
              <div onKeyDownCapture={handleKeyDownCapture} className="relative max-w-xl w-full">
                <div className="flex items-center gap-2 px-4 py-2 border rounded-full bg-white shadow-sm" style={{ borderColor: 'rgb(253,236,231)' }}>
                  <IndexSearchInput query={query} setQuery={setQuery} isSearchLoading={isSearchLoading} />
                </div>
              </div>
            </div>

            {/* 右侧：空白区域保持平衡 */}
            <div className='w-0 flex-1'></div>
          </div>
          <div className='w-full border-b border-gray-200' />
        </div>

        {/* 主体内容区域 - 左右并排布局 */}
        <div className="max-w-7xl mx-auto w-full px-8">
          {/* 主标题 - 左对齐，跨越整个宽度 */}
          <h1 className="text-3xl sm:text-4xl font-bold mb-8 text-left">Rust Ecosystem Updates</h1>
          
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
            {/* Latest Crate Updates */}
            <section className="flex flex-col">
              <h2 className="text-2xl font-bold mb-4 pb-2 border-b" style={{ borderColor: 'rgb(253,236,231)' }}>Latest Crate Updates</h2>
              <div className="flex-grow overflow-x-auto rounded-lg border bg-white shadow-sm" style={{ borderColor: 'rgb(253,236,231)' }}>
                <table className="w-full text-left h-full">
                  <thead style={{ backgroundColor: 'rgb(253,236,231)' }}>
                    <tr>
                      <th className="px-6 py-3 text-sm font-semibold">Crate</th>
                      <th className="px-6 py-3 text-sm font-semibold">Version</th>
                      <th className="px-6 py-3 text-sm font-semibold">Updated</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y" style={{ borderColor: 'rgb(253,236,231)' }}>
                    <tr>
                      <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">serde</td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">1.0.197</td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">2024-07-26</td>
                    </tr>
                    <tr>
                      <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">tokio</td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">1.38.1</td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">2024-07-25</td>
                    </tr>
                    <tr>
                      <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">actix-web</td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">4.8.0</td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">2024-07-24</td>
                    </tr>
                    <tr>
                      <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">reqwest</td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">0.12.5</td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">2024-07-23</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </section>

            {/* Latest Rust CVEs */}
            <section className="flex flex-col">
              <h2 className="text-2xl font-bold mb-4 pb-2 border-b" style={{ borderColor: 'rgb(253,236,231)' }}>Latest Rust CVEs</h2>
              <div className="flex-grow overflow-x-auto rounded-lg border bg-white shadow-sm" style={{ borderColor: 'rgb(253,236,231)' }}>
                <table className="w-full text-left h-full">
                  <thead style={{ backgroundColor: 'rgb(253,236,231)' }}>
                    <tr>
                      <th className="px-6 py-3 text-sm font-semibold">CVE ID</th>
                      <th className="px-6 py-3 text-sm font-semibold">Description</th>
                      <th className="px-6 py-3 text-sm font-semibold">Date</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y" style={{ borderColor: 'rgb(253,236,231)' }}>
                    {cveLoading && (
                      <tr>
                        <td className="px-6 py-4 text-sm text-gray-600" colSpan={3}>Loading...</td>
                      </tr>
                    )}
                    {!cveLoading && cveError && (
                      <tr>
                        <td className="px-6 py-4 text-sm text-red-500" colSpan={3}>{cveError}</td>
                      </tr>
                    )}
                    {!cveLoading && !cveError && latestCves.length === 0 && (
                      <tr>
                        <td className="px-6 py-4 text-sm text-gray-500" colSpan={3}>No data</td>
                      </tr>
                    )}
                    {!cveLoading && !cveError && latestCves.map((item) => (
                      <tr key={item.id}>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">
                          <Link
                            href={`/${router.query.org || 'org'}/rust/rust-ecosystem/ecosystem-cve/cve-info?cveId=${encodeURIComponent(item.id)}`}
                            className="text-gray-900 hover:text-blue-600 hover:underline"
                          >
                            {item.id}
                          </Link>
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">{item.subtitle || item.description}</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">{item.issued}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </section>
          </div>
        </div>
      </div>
    </>
  )
}

CratesproPage.getProviders = (page, pageProps) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default CratesproPage

import Head from 'next/head'
import { useState } from 'react'
import { IndexSearchInput } from '@/components/IndexPages/components'
import Image from 'next/image'

const newsList = [
  {
    date: '11-06',
    year: '2025',
    title: '聚焦Rust数字浪潮奔涌的时代，书写着未来技术的恢弘篇章。',
    tags: ['行业新闻', '媒体报道', 'Rust'],
    hot: true,
    summary: '在数字浪潮奔涌的时代，北京举办的Rust China Tour再度回归，汇聚全球开发者与科技领袖的视野，开启一场新篇章的盛宴。',
  },
  {
    date: '10-26',
    year: '2025',
    title: 'Rust生态系统持续壮大，创新应用层出不穷。',
    tags: ['行业新闻', '媒体报道', 'Rust'],
    hot: false,
    summary: 'Rust生态系统在2025年迎来新一轮爆发，众多企业和开发者积极参与，推动技术创新与产业升级。',
  },
  {
    date: '09-14',
    year: '2025',
    title: 'Rust助力数字基础设施升级，安全与性能并重。',
    tags: ['行业新闻', '媒体报道', 'Rust'],
    hot: false,
    summary: '随着数字经济的发展，Rust语言以其安全性和高性能成为基础设施建设的首选，广受业界关注。',
  },
  {
    date: '01-22',
    year: '2025',
    title: 'Rust社区活动精彩纷呈，开发者热情高涨。',
    tags: ['行业新闻', '媒体报道', 'Rust'],
    hot: true,
    summary: '2025年初，Rust社区举办多场技术沙龙和线上研讨会，吸引了大量开发者参与，推动知识分享与技术交流。',
  },
  {
    date: '11-06',
    year: '2025',
    title: 'Rust技术赋能智能制造，推动产业升级。',
    tags: ['行业新闻', '媒体报道', 'Rust'],
    hot: false,
    summary: '智能制造领域引入Rust技术，提升了系统的稳定性与安全性，助力企业实现数字化转型。',
  }, 
  {
    date: '01-22',
    year: '2025',
    title: 'Rust社区活动精彩纷呈，开发者热情高涨。',
    tags: ['行业新闻', '媒体报道', 'Rust'],
    hot: true,
    summary: '2025年初，Rust社区举办多场技术沙龙和线上研讨会，吸引了大量开发者参与，推动知识分享与技术交流。',
  },
  {
    date: '01-22',
    year: '2025',
    title: 'Rust社区活动精彩纷呈，开发者热情高涨。',
    tags: ['行业新闻', '媒体报道', 'Rust'],
    hot: true,
    summary: '2025年初，Rust社区举办多场技术沙龙和线上研讨会，吸引了大量开发者参与，推动知识分享与技术交流。',
  },
  {
    date: '10-26',
    year: '2025',
    title: 'Rust生态系统持续壮大，创新应用层出不穷。',
    tags: ['行业新闻', '媒体报道', 'Rust'],
    hot: false,
    summary: 'Rust生态系统在2025年迎来新一轮爆发，众多企业和开发者积极参与，推动技术创新与产业升级。',
  },
  {
    date: '09-14',
    year: '2025',
    title: 'Rust助力数字基础设施升级，安全与性能并重。',
    tags: ['行业新闻', '媒体报道', 'Rust'],
    hot: false,
    summary: '随着数字经济的发展，Rust语言以其安全性和高性能成为基础设施建设的首选，广受业界关注。',
  },
  {
    date: '11-06',
    year: '2025',
    title: 'Rust技术赋能智能制造，推动产业升级。',
    tags: ['行业新闻', '媒体报道', 'Rust'],
    hot: false,
    summary: '智能制造领域引入Rust技术，提升了系统的稳定性与安全性，助力企业实现数字化转型。',
  }, 
  {
    date: '01-22',
    year: '2025',
    title: 'Rust社区活动精彩纷呈，开发者热情高涨。',
    tags: ['行业新闻', '媒体报道', 'Rust'],
    hot: true,
    summary: '2025年初，Rust社区举办多场技术沙龙和线上研讨会，吸引了大量开发者参与，推动知识分享与技术交流。',
  },
  {
    date: '01-22',
    year: '2025',
    title: 'Rust社区活动精彩纷呈，开发者热情高涨。',
    tags: ['行业新闻', '媒体报道', 'Rust'],
    hot: true,
    summary: '2025年初，Rust社区举办多场技术沙龙和线上研讨会，吸引了大量开发者参与，推动知识分享与技术交流。',
  },
  {
    date: '10-26',
    year: '2025',
    title: 'Rust生态系统持续壮大，创新应用层出不穷。',
    tags: ['行业新闻', '媒体报道', 'Rust'],
    hot: false,
    summary: 'Rust生态系统在2025年迎来新一轮爆发，众多企业和开发者积极参与，推动技术创新与产业升级。',
  },
  {
    date: '09-14',
    year: '2025',
    title: 'Rust助力数字基础设施升级，安全与性能并重。',
    tags: ['行业新闻', '媒体报道', 'Rust'],
    hot: false,
    summary: '随着数字经济的发展，Rust语言以其安全性和高性能成为基础设施建设的首选，广受业界关注。',
  },
  {
    date: '11-06',
    year: '2025',
    title: 'Rust技术赋能智能制造，推动产业升级。',
    tags: ['行业新闻', '媒体报道', 'Rust'],
    hot: false,
    summary: '智能制造领域引入Rust技术，提升了系统的稳定性与安全性，助力企业实现数字化转型。',
  }, 
  {
    date: '01-22',
    year: '2025',
    title: 'Rust社区活动精彩纷呈，开发者热情高涨。',
    tags: ['行业新闻', '媒体报道', 'Rust'],
    hot: true,
    summary: '2025年初，Rust社区举办多场技术沙龙和线上研讨会，吸引了大量开发者参与，推动知识分享与技术交流。',
  },
  {
    date: '01-22',
    year: '2025',
    title: 'Rust社区活动精彩纷呈，开发者热情高涨。',
    tags: ['行业新闻', '媒体报道', 'Rust'],
    hot: true,
    summary: '2025年初，Rust社区举办多场技术沙龙和线上研讨会，吸引了大量开发者参与，推动知识分享与技术交流。',
  },
  {
    date: '10-26',
    year: '2025',
    title: 'Rust生态系统持续壮大，创新应用层出不穷。',
    tags: ['行业新闻', '媒体报道', 'Rust'],
    hot: false,
    summary: 'Rust生态系统在2025年迎来新一轮爆发，众多企业和开发者积极参与，推动技术创新与产业升级。',
  },
  {
    date: '09-14',
    year: '2025',
    title: 'Rust助力数字基础设施升级，安全与性能并重。',
    tags: ['行业新闻', '媒体报道', 'Rust'],
    hot: false,
    summary: '随着数字经济的发展，Rust语言以其安全性和高性能成为基础设施建设的首选，广受业界关注。',
  },
  
]

const tagColor = (tag: string) => {
  if (tag === '行业新闻') return 'bg-blue-100 text-blue-600'
  if (tag === '媒体报道') return 'bg-green-100 text-green-600'
  if (tag === 'Rust') return 'bg-orange-100 text-orange-600'
  return 'bg-gray-100 text-gray-600'
}

export default function RustNewsPage() {
  const [search, setSearch] = useState('')
  const isSearchLoading = false

  return (
    <>
      <Head>
        <title>Rust News</title>
      </Head>
      <div className="min-h-screen h-auto w-full bg-white px-0 py-0">
        {/* 搜索栏 */}
        <div
          className="flex items-center border-b border-gray-200 bg-white w-full sticky top-0 z-20"
          style={{
            height: 53,
            flexShrink: 0,
            marginTop: 0,
            marginBottom: 0,
            paddingLeft: 32,
            paddingRight: 32,
          }}
        >
          <div className="flex-1 max-w-xl">
            <IndexSearchInput query={search} setQuery={setSearch} isSearchLoading={isSearchLoading} />
          </div>
        </div>
        {/* 主标题 */}
        <div className="max-w-6xl mx-auto mt-6">
          <h1 className="text-6xl font-black text-[#222]">Rust News</h1>
          <div
            style={{
              width: 360,
              height: 14,
              flexShrink: 0,
              borderRadius: 2,
              background: '#3E63DD',
              marginTop: 8,
              marginBottom: 32,
            }}
          />
        </div>
        {/* 新闻列表 */}
        <div className="max-w-6xl mx-auto flex flex-col gap-8">
          {newsList
            .filter(item => item.title.includes(search) || item.summary.includes(search))
            .map((item) => (
            <div
              key={item.date + '-' + item.title}
              className="flex bg-white rounded-2xl shadow-sm border border-gray-200 px-8 py-6 items-center gap-8"
            >
              {/* 日期 */}
              <div className="flex flex-col items-center justify-center min-w-[70px]">
                <span className="text-2xl font-bold text-gray-800 leading-none">{item.date}</span>
                <span className="text-base text-gray-400 mt-1">{item.year}</span>
              </div>
              {/* 竖线分割 */}
              <div className="h-16 w-px bg-gray-200 mx-1" />
              {/* 内容 */}
              <div className="flex-1 flex flex-col">
                <div className="flex items-center">
                  <span className="text-xl font-bold text-gray-900">{item.title}</span>
                  {item.hot && <span className="ml-2 text-red-500 text-xl">🔥</span>}
                </div>
                <div className="flex gap-2 mt-2">
                  {item.tags.map(tag => (
                    <span
                      key={tag}
                      className={`px-2 py-0.5 rounded text-xs font-semibold ${tagColor(tag)}`}
                    >
                      {tag}
                    </span>
                  ))}
                </div>
                <div className="text-gray-500 text-sm mt-2 line-clamp-2">{item.summary}</div>
              </div>
              {/* 详情按钮 */}
              <div className="flex flex-col items-end justify-between h-full ml-4">
                <button className="bg-blue-600 text-white px-6 py-2 rounded-lg font-semibold shadow hover:bg-blue-700 transition">
                  Details
                </button>
              </div>
            </div>
          ))}
        </div>
        {/* 右下角up-icon */}
        <button
          onClick={() => {
            window.scrollTo({ top: 0, behavior: 'smooth' });
            const main = document.querySelector('#__next');
            
            if (main) {
              main.scrollTo({ top: 0, behavior: 'smooth' });
            }
          }}
          style={{
            position: 'fixed',
            right: 350,
            bottom: 40,
            zIndex: 50,
            background: 'none',
            border: 'none',
            padding: 0,
            cursor: 'pointer'
          }}
          aria-label="回到顶部"
        >
          <Image src="/rust/rust-news/up-icon.png" alt="回到顶部" width={48} height={48} />
        </button>
      </div>
    </>
  )
}
import { useEffect, useRef, useState } from 'react'

interface Props {
  url: string
  onLoad?: (animationItem: any) => void
  onError?: () => void
  onFrame?: (frame: number) => void
  className?: string
}

export function Lottie(props: Props) {
  const { url, onLoad, onError, onFrame, className = 'w-full h-full' } = props
  const ref = useRef(null)
  const [animationItem, setAnimationItem] = useState<any>(null)
  const [loaded, setLoaded] = useState(false)
  const [_frame, setFrame] = useState(0)
  // lottie-web (pulled in by react-lottie-player) touches `document` at import time, which
  // breaks the Next.js production build during page-data collection. Load the player lazily
  // on the client so the module never enters the SSR import graph.
  const [LottieLight, setLottieLight] = useState<any>(null)

  useEffect(() => {
    let mounted = true

    import('react-lottie-player/dist/LottiePlayerLight').then((mod) => {
      if (mounted) setLottieLight(() => mod.default)
    })

    return () => {
      mounted = false
    }
  }, [])

  useEffect(() => {
    if (loaded) {
      const player = ref.current as any

      setAnimationItem(player)
      onLoad?.(player)
    }
  }, [ref, loaded, onLoad])

  const handleLoad = () => {
    setLoaded(true)
  }

  const handleEnterFrame = () => {
    const percentage = (animationItem?.currentFrame / animationItem?.totalFrames) * 100

    setFrame(percentage)
    onFrame?.(percentage)
  }

  if (!LottieLight) return null

  return (
    <LottieLight
      ref={ref}
      path={url}
      play
      loop
      onEnterFrame={handleEnterFrame}
      onLoad={handleLoad}
      onError={onError}
      className={className}
    />
  )
}

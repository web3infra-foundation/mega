declare namespace JSX {
  interface IntrinsicElements {
    'lottie-player': any
  }
}

declare module 'prismjs/components/prism-*' {
  const content: any

  export default content
}

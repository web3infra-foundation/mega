import { ThemeProvider as Theme } from 'next-themes'

interface Props {
  children: React.ReactNode
}

export function ThemeProvider({ children }: Props) {
  return (
    <Theme attribute='class' defaultTheme='light' enableSystem={false}>
      {children}
    </Theme>
  )
}

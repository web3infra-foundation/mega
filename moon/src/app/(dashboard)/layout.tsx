import { ApplicationLayout } from './application-layout'

export default function DashboardLayout({
    children,
  }: {
    children: React.ReactNode
  }) {
    return <ApplicationLayout>{children}</ApplicationLayout>
  }
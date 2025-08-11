import { useEffect } from 'react';
import { ProfileDisplay } from 'components/UserSettings/ProfileDisplay';
import { ProfileSecurity } from 'components/UserSettings/ProfileSecurity';
import { TwoFactorAuthentication } from 'components/UserSettings/TwoFactorAuthentication';
import Head from 'next/head';
import { CopyCurrentUrl } from '@/components/CopyCurrentUrl';
import AuthAppProviders from '@/components/Providers/AuthAppProviders';
import SSHKeys from '@/components/Setting/SSHKeys';
import PersonalToken from '@/components/Setting/PersonalToken';
import { ThemePicker } from '@/components/ThemePicker';
import { Behaviors } from '@/components/UserSettings/Behaviors';
import { NotificationSettings } from '@/components/UserSettings/Notifications/NotificationSettings';
import { PushNotificationSettings } from '@/components/UserSettings/Notifications/PushNotificationSettings';
import { NotificationSchedule } from '@/components/UserSettings/NotificationSchedule';
import { UserSettingsPageWrapper } from '@/components/UserSettings/PageWrapper';
import { PersonalCallLinks } from '@/components/UserSettings/PersonalCallLinks';
import { SlackNotificationSettings } from '@/components/UserSettings/SlackNotificationSettings';
import { Timezone } from '@/components/UserSettings/Timezone'
import { PageWithProviders } from '@/utils/types';

const UserSettingsPage: PageWithProviders<any> = () => {
  useEffect(() => {
    const hash = window.location.hash

    if (!hash) return

    const element = document.querySelector(hash)

    if (element) {
      element.scrollIntoView({ behavior: 'auto' })
    }
  }, [])

  return (
    <>
      <Head>
        <title>Account settings</title>
      </Head>

      <CopyCurrentUrl />

      <UserSettingsPageWrapper>
        <ProfileDisplay />
        <Timezone />
        <SSHKeys />
        <PersonalToken />
        <PersonalCallLinks />
        <ThemePicker />
        <Behaviors />
        <PushNotificationSettings />
        <NotificationSettings />
        <NotificationSchedule />
        <SlackNotificationSettings />
        <ProfileSecurity />
        <TwoFactorAuthentication />
      </UserSettingsPageWrapper>
    </>
  )
}

UserSettingsPage.getProviders = (page, pageProps) => {
  return <AuthAppProviders {...pageProps}>{page}</AuthAppProviders>
}

export default UserSettingsPage

'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { useTranslations } from 'next-intl';
import { locales } from '@/i18n/request';

export default function LanguageSwitcher() {
    const router = useRouter();
    const [currentLocale, setCurrentLocale] = useState<string>('en');

    // Get current locale from cookie on component mount
    useEffect(() => {
        const storedLocale = document.cookie
            .split('; ')
            .find(row => row.startsWith('NEXT_LOCALE='))
            ?.split('=')[1] || 'en';

        setCurrentLocale(storedLocale);
    }, []);

    const handleLanguageChange = (newLocale: string) => {
        // Set a cookie with the new locale
        document.cookie = `NEXT_LOCALE=${newLocale}; path=/; max-age=${60 * 60 * 24 * 365}`;

        // Update state
        setCurrentLocale(newLocale);

        // Reload the page to apply the new language
        router.refresh();
    };

    // Language names in their native language - keep these hardcoded
    // since they should display in their own language regardless of current locale
    const LANGUAGE_NAMES: Record<string, string> = {
        en: 'English',
        zh: '中文'
    };

    return (
        <div className="flex items-center space-x-2">
            {locales.map((locale) => (
                <button
                    key={locale}
                    onClick={() => handleLanguageChange(locale)}
                    className={`px-3 py-1 rounded text-sm transition-colors ${currentLocale === locale
                        ? 'bg-blue-600 text-white'
                        : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
                        } text-nowrap`}
                    aria-current={currentLocale === locale ? 'true' : 'false'}
                    title={`Switch to ${LANGUAGE_NAMES[locale]}`}
                >
                    {LANGUAGE_NAMES[locale]}
                </button>
            ))}
        </div>
    );
}

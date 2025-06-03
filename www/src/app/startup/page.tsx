'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { initializeWebApp } from '@/lib/api-client';
import { useTranslations } from 'next-intl';

export default function StartupPage() {
    const router = useRouter();
    const [status, setStatus] = useState('Initializing...');
    const [error, setError] = useState<string | null>(null);
    const t = useTranslations('Startup');

    useEffect(() => {
        const performInitialization = async () => {
            try {
                setStatus(t('status_initializing'));
                console.log('Attempting webapp initialization...');
                const result = await initializeWebApp();
                console.log('Initialization API result:', result);

                if (result.success) {
                    setStatus(t('status_success'));
                    // Set a flag in localStorage to indicate initialization is complete
                    localStorage.setItem('webappInitialized', 'true');
                    console.log('Initialization successful, redirecting to home...');
                    // Redirect to the home page after a short delay
                    // Adjust delay based on whether GitHub load was triggered
                    const redirectDelay = result.githubLoadTriggered ? 0 : 1000;
                    setTimeout(() => router.push('/'), redirectDelay);
                } else {
                    console.error('Initialization failed:', result.message);
                    setError(result.message || t('error_unknown'));
                    setStatus(t('status_failed'));
                }
            } catch (err: any) {
                console.error('Error during initialization:', err);
                setError(err.message || t('error_exception'));
                setStatus(t('status_failed'));
            }
        };

        performInitialization();
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [router]); // Removed t from dependencies as it might cause loops if translations change

    return (
        <div className="flex flex-col items-center justify-center min-h-screen bg-gray-100 dark:bg-gray-900">
            <div className="p-8 bg-white dark:bg-gray-800 rounded-lg shadow-md text-center">
                <h1 className="text-2xl font-bold mb-4 text-gray-900 dark:text-gray-100">{t('title')}</h1>
                <p className="text-gray-700 dark:text-gray-300 mb-4">
                    {t('description')}
                </p>
                <div className="mt-4">
                    <p className="text-lg font-medium text-gray-900 dark:text-gray-100">{status}</p>
                    {error && (
                        <p className="mt-2 text-sm text-red-600 dark:text-red-400">Error: {error}</p>
                    )}
                </div>
                {/* Optional: Add a spinner */}
                {status === t('status_initializing') && (
                    <div className="mt-4 animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500 mx-auto"></div>
                )}
            </div>
        </div>
    );
}

// Add basic translations for this page - these should ideally be in your translation files
// For example, in messages/en.json:
/*
{
  "Startup": {
    "title": "Initializing Subconverter",
    "description": "Please wait while we set things up for the first time. This might take a moment...",
    "status_initializing": "Initializing...",
    "status_success": "Initialization Complete! Redirecting...",
    "status_failed": "Initialization Failed",
    "error_unknown": "An unknown error occurred during initialization.",
    "error_exception": "An exception occurred during initialization."
  }
}
*/ 
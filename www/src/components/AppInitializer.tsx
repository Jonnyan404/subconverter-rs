'use client';

import { useEffect, useState } from 'react';
import { usePathname, useRouter } from 'next/navigation';

// Path for the startup page
const STARTUP_PATH = '/startup';
// Key for localStorage flag
const INIT_FLAG_KEY = 'webappInitialized';

export default function AppInitializer({ children }: { children: React.ReactNode }) {
    const router = useRouter();
    const pathname = usePathname();
    const [isInitialized, setIsInitialized] = useState<boolean | null>(null); // null initially, true/false after check

    useEffect(() => {
        // Check localStorage only on the client side
        const initialized = localStorage.getItem(INIT_FLAG_KEY) === 'true';
        setIsInitialized(initialized);

        console.log(`AppInitializer: Initialized flag = ${initialized}`);

        // If not initialized and not already on the startup page, redirect
        if (!initialized && pathname !== STARTUP_PATH) {
            console.log(`AppInitializer: Redirecting to ${STARTUP_PATH}`);
            router.replace(STARTUP_PATH);
        } else if (initialized && pathname === STARTUP_PATH) {
            // If somehow initialized but still on startup, redirect home
            console.log(`AppInitializer: Already initialized, redirecting from ${STARTUP_PATH} to /`);
            router.replace('/');
        }
    }, [pathname, router]);

    // Don't render children until the initialization check is complete and successful,
    // or if we are already on the startup page (let it handle its own rendering)
    if (isInitialized === null || (!isInitialized && pathname !== STARTUP_PATH) || (isInitialized && pathname === STARTUP_PATH)) {
        // Render minimal content or a loading indicator while checking/redirecting
        // Returning null prevents rendering children during the redirect flicker
        return null;
    }

    // Render children only if initialized and not on the startup page
    return <>{children}</>;
} 
// This file provides TypeScript interfaces for the subconverter WASM module

// Import the WASM module and its types directly
import * as subconverterWasm from 'subconverter-wasm';
import type { DirectoryEntry, FileAttributes } from 'subconverter-wasm';

// Export the types for use elsewhere
export type { DirectoryEntry, FileAttributes };

// BigInt.prototype.toJSON = function () {
(BigInt.prototype as any).toJSON = function () {
    return { $bigint: this.toString() };
};

// Debug flag from environment
const isDebug = process.env.WASM_DEBUG === 'true';
const deployEnv = process.env.DEPLOY_ENV || 'unknown';

type SubconverterWasm = typeof subconverterWasm;

// WASM module singleton to avoid re-initialization
let wasmModule: SubconverterWasm | null = null;
let initPromise: Promise<SubconverterWasm> | null = null;

/**
 * Determines if we're in a specific deployment environment
 */
export function getDeploymentEnv(): string {

    // Check for environment variable set in webpack
    if (deployEnv !== 'unknown') {
        return deployEnv;
    }

    // Netlify detection
    if (
        process.env.NETLIFY === 'true' ||
        process.env.CONTEXT === 'production' ||
        process.env.NETLIFY_LOCAL === 'true' ||
        (process.env.DEPLOY_URL && process.env.DEPLOY_URL.includes('netlify'))
    ) {
        return 'netlify';
    }

    // Vercel detection
    if (process.env.VERCEL === 'true') {
        return 'vercel';
    }

    return 'standard';
}

/**
 * Determines if we're in a Next.js server environment
 */
export function isNextJsServer(): boolean {
    return process.env.NEXT_RUNTIME === 'nodejs' ||
        process.env.NODE_ENV === 'production' && typeof window === 'undefined';
}

/**
 * Check if we're running in the Netlify environment
 */
export function isNetlifyEnvironment(): boolean {
    return typeof process !== 'undefined' &&
        process.env.NETLIFY === 'true' ||
        (process.cwd && process.cwd() === '/var/task');
}

/**
 * Initialize the WASM module using the pre-generated wasm-bindgen bindings
 */
export async function initWasm(): Promise<typeof subconverterWasm> {
    try {
        console.log('🔄 Initializing WASM module using wasm-bindgen bindings...');

        // Check environment
        const isDev = process.env.NODE_ENV === 'development';
        console.log(`Environment: ${isDev ? 'Development' : 'Production'}`);
        console.log(`Deployment: ${getDeploymentEnv()}`);

        // Initialize necessary hooks
        if (typeof subconverterWasm.init_panic_hook === 'function') {
            console.log('Initializing panic hook...');
            subconverterWasm.init_panic_hook();
        }

        if (typeof subconverterWasm.init_wasm_logging === 'function') {
            console.log('Initializing WASM logging...');
            subconverterWasm.init_wasm_logging(process.env.NODE_ENV === 'development' ? 'trace' : 'info');
        }

        if (typeof subconverterWasm.admin_init_kv_bindings_js === 'function') {
            console.log('Initializing KV bindings...');
            subconverterWasm.admin_init_kv_bindings_js();
        }

        // Log available methods if in debug mode
        if (isDebug) {
            const methodNames = Object.getOwnPropertyNames(subconverterWasm)
                .filter(k => typeof (subconverterWasm as any)[k] === 'function');
            console.log('Available WASM methods:', methodNames.join(', '));
        }

        console.log('Successfully initialized WASM module');
        return subconverterWasm;
    } catch (error) {
        console.error('❌ Unhandled error in initWasm:', error);
        throw error;
    }
}

/**
 * Shared singleton loader for the WASM module
 * This ensures the module is only loaded once across all API endpoints
 * @param context Optional context string for logging
 */
export async function loadWasmSingleton(context: string = 'API'): Promise<SubconverterWasm> {
    if (!initPromise) {
        console.log(`[${context}] Initializing WASM module singleton...`);
        initPromise = initWasm()
            .then((module) => {
                wasmModule = module;
                console.log(`[${context}] WASM singleton initialized successfully`);
                return module;
            })
            .catch((err) => {
                console.error(`[${context}] Failed to load or initialize WASM:`, err);
                initPromise = null; // Reset promise on failure
                throw err;
            });
    }
    return initPromise;
}

// Initialize the module
if (typeof subconverterWasm.init_panic_hook === 'function') {
    subconverterWasm.init_panic_hook();
    console.log('Initialized subconverter-wasm panic hook');
}

if (typeof subconverterWasm.admin_init_kv_bindings_js === 'function') {
    subconverterWasm.admin_init_kv_bindings_js();
    console.log('Initialized subconverter-wasm kv bindings');
}

// Log environment information
const env = getDeploymentEnv();
console.log(`Running in ${env} environment`);
console.log(`Is Next.js server: ${isNextJsServer()}`);
console.log(`Is Netlify: ${isNetlifyEnvironment()}`);

// Export all functions from the module
export * from 'subconverter-wasm';

// Call a dummy function to test the module is loaded
console.log('Testing WASM module loaded successfully');
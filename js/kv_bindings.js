// Example: kv_bindings.js (or integrate into your Edge Function handler)
// Ensure @vercel/kv or @netlify/blobs is installed in your project dependencies (package.json)
// Ensures fallback for local development when neither Vercel KV nor Netlify Blobs are available

// --- Configuration ---
const CURRENT_STORAGE_VERSION = 1; // Increment this when making breaking changes
const VERCEL_KV_PREFIX = `v${CURRENT_STORAGE_VERSION}`;
const NETLIFY_STORE_NAME = `subconverter-data-v${CURRENT_STORAGE_VERSION}`;

// ---------------------

// Expose the localStorageMap for debugging
let localStorageMap = new Map(); // Local in-memory fallback
let kv; // Lazy load KV
let isNetlifyBlobs = false; // Flag to track if we're using Netlify Blobs

// Environment variable cache to avoid repeated lookups
let envCache = new Map();

function isNetlifyEnvironment() {
    return typeof process !== 'undefined' &&
        process.env.NETLIFY === 'true' ||
        process.env.NETLIFY_BLOBS_CONTEXT != undefined ||
        (process.cwd && process.cwd() === '/var/task');
}

// Function to read environment variables from various runtimes
// This is needed because std::env::var doesn't work in WebAssembly
function getenv(name, defaultValue = "") {
    // Check cache first
    if (envCache.has(name)) {
        return envCache.get(name);
    }

    let value = defaultValue;

    try {
        // Check for Node.js process.env
        if (typeof process !== 'undefined' && process.env) {
            if (name in process.env) {
                value = process.env[name];
            }
        }
        // Check for browser environment
        else if (typeof window !== 'undefined') {
            // Try for environment variables set via window.__ENV__ (common pattern)
            if (window.__ENV__ && name in window.__ENV__) {
                value = window.__ENV__[name];
            }
        }
        // Cloudflare Workers and other edge runtimes might have their own way
        // For example, Cloudflare Workers use env bindings set during deployment
    } catch (error) {
        console.warn(`Error reading environment variable ${name}:`, error);
    }

    // Cache the result
    envCache.set(name, value);
    return value;
}

async function getKv() {
    if (!kv) {
        try {
            // Check for Vercel KV environment
            if (typeof process !== 'undefined' &&
                process.env.KV_REST_API_URL &&
                process.env.KV_REST_API_TOKEN) {
                const vercelKv = require('@vercel/kv');
                const baseKv = vercelKv.kv; // Get the base client
                console.log("Using Vercel KV for storage (version prefix: ", VERCEL_KV_PREFIX, ")");

                // Create adapter with version prefixing
                kv = {
                    _baseKv: baseKv,
                    get: (key) => baseKv.get(`${VERCEL_KV_PREFIX}/${key}`),
                    set: (key, value) => baseKv.set(`${VERCEL_KV_PREFIX}/${key}`, value),
                    exists: (key) => baseKv.exists(`${VERCEL_KV_PREFIX}/${key}`),
                    del: (key) => baseKv.del(`${VERCEL_KV_PREFIX}/${key}`),
                    scan: async (cursor, options = {}) => {
                        const { match = "*", count = 10 } = options;
                        // Adapt the match pattern to include the prefix
                        const prefixedMatch = `${VERCEL_KV_PREFIX}/${match}`;
                        const [nextCursor, keys] = await baseKv.scan(cursor, { match: prefixedMatch, count });
                        // Remove prefix from returned keys
                        const unprefixedKeys = keys.map(k => k.startsWith(VERCEL_KV_PREFIX + '/') ? k.substring(VERCEL_KV_PREFIX.length + 1) : k);
                        return [nextCursor, unprefixedKeys];
                    }
                };

            }
            // Check for Netlify Blobs environment
            else if (isNetlifyEnvironment()) {
                try {
                    const { getStore } = require('@netlify/blobs');
                    const store = getStore(NETLIFY_STORE_NAME); // Use versioned store name
                    isNetlifyBlobs = true;
                    console.log("Using Netlify Blobs for storage (store: ", NETLIFY_STORE_NAME, ")");

                    // Create adapter to match Vercel KV interface
                    kv = {
                        // Get a value by key
                        get: async (key) => {
                            try {
                                const value = await store.get(key, { type: 'arrayBuffer' });
                                return value ? new Uint8Array(value) : null;
                            } catch (error) {
                                if (error.message.includes('not found')) {
                                    return null;
                                }
                                throw error;
                            }
                        },
                        // Set a key-value pair
                        set: async (key, value) => {
                            await store.set(key, value);
                            return "OK";
                        },
                        // Check if a key exists
                        exists: async (key) => {
                            try {
                                const metadata = await store.getMetadata(key);
                                return metadata ? 1 : 0;
                            } catch (error) {
                                return 0;
                            }
                        },
                        // Scan keys with pattern matching - Netlify Blobs prefix scan is efficient
                        scan: async (cursor, options = {}) => {
                            const { match = "*", count = 10 } = options;
                            // Netlify list uses prefix, not glob. We'll filter later if needed.
                            const prefix = match.endsWith('*') ? match.slice(0, -1) : '';
                            // We ignore the cursor for Netlify list as it returns all matching keys
                            // Pagination would need custom implementation if required beyond simple prefix listing
                            const list = await store.list({ prefix });
                            let keys = list.blobs.map(blob => blob.key);
                            // If a more complex pattern was given, filter client-side
                            if (match !== "*" && !match.endsWith('*')) {
                                const pattern = match.replace(/\*/g, ".*");
                                const regex = new RegExp(`^${pattern}$`);
                                keys = keys.filter(key => regex.test(key));
                            }
                            // Return result mimicking Vercel KV scan (cursor 0 means done for this simple impl)
                            return ['0', keys.slice(0, count)];
                        },
                        // Store reference for direct access
                        _store: store,
                        // Delete a key
                        del: async (key) => {
                            try {
                                await store.delete(key);
                                return 1;
                            } catch (error) {
                                console.error(`Error deleting key ${key}:`, error);
                                return 0;
                            }
                        }
                    };
                } catch (error) {
                    console.warn("Error initializing Netlify Blobs:", error);
                    throw error; // Let the fallback handle it
                }
            } else {
                // Use local storage fallback (remains unversioned)
                console.log("No KV storage environment detected, using in-memory fallback (unversioned)");
                // Create an in-memory implementation that mimics the Vercel KV API
                kv = {
                    get: async (key) => localStorageMap.get(key) || null,
                    set: async (key, value) => { localStorageMap.set(key, value); return "OK"; },
                    exists: async (key) => localStorageMap.has(key) ? 1 : 0,
                    scan: async (cursor, options = {}) => {
                        const { match = "*", count = 10 } = options;
                        const pattern = match.replace(/\*/g, ".*");
                        const regex = new RegExp(`^${pattern}$`);
                        const allKeys = [...localStorageMap.keys()];
                        const matchingKeys = allKeys.filter(key => regex.test(key));
                        const startIndex = parseInt(cursor) || 0;
                        const endIndex = Math.min(startIndex + count, matchingKeys.length);
                        const keys = matchingKeys.slice(startIndex, endIndex);
                        const nextCursor = endIndex < matchingKeys.length ? String(endIndex) : '0';
                        return [nextCursor, keys];
                    },
                    del: async (key) => localStorageMap.delete(key) ? 1 : 0
                };
            }
        } catch (error) {
            // Error during initialization, use fallback (remains unversioned)
            console.warn("Error initializing storage, using in-memory fallback (unversioned):", error);
            // Create an in-memory implementation that mimics the Vercel KV API
            kv = {
                get: async (key) => localStorageMap.get(key) || null,
                set: async (key, value) => { localStorageMap.set(key, value); return "OK"; },
                exists: async (key) => localStorageMap.has(key) ? 1 : 0,
                scan: async (cursor, options = {}) => {
                    const { match = "*", count = 10 } = options;
                    const pattern = match.replace(/\*/g, ".*");
                    const regex = new RegExp(`^${pattern}$`);
                    const allKeys = [...localStorageMap.keys()];
                    const matchingKeys = allKeys.filter(key => regex.test(key));
                    const startIndex = parseInt(cursor) || 0;
                    const endIndex = Math.min(startIndex + count, matchingKeys.length);
                    const keys = matchingKeys.slice(startIndex, endIndex);
                    const nextCursor = endIndex < matchingKeys.length ? String(endIndex) : '0';
                    return [nextCursor, keys];
                },
                del: async (key) => localStorageMap.delete(key) ? 1 : 0
            };
        }
    }
    return kv;
}

// Helper to handle potential null from kv.get
// Both Vercel KV and Netlify Blobs may store raw bytes differently
// For Vercel KV, it stores raw bytes as base64 strings when using the REST API directly
// For Netlify Blobs, we request arrayBuffer type and convert to Uint8Array
async function kv_get(key) {
    try {
        const kvClient = await getKv();
        const value = await kvClient.get(key);

        if (value instanceof ArrayBuffer) {
            return new Uint8Array(value);
        } else if (ArrayBuffer.isView(value) && !(value instanceof DataView)) {
            // Handles Uint8Array from in-memory fallback or Netlify Blobs
            return value;
        } else if (typeof value === 'string') {
            // Vercel KV might return a string for non-binary data
            return value;
        }

        return value === null ? undefined : value;
    } catch (error) {
        console.error(`KV get error for ${key}:`, error);
        // Re-throw or return specific error indicator if needed
        throw new Error(`Failed to get key ${key}: ${error.message}`);
    }
}

async function kv_get_text(key) {
    try {
        const kvClient = await getKv();
        // Vercel KV, Netlify Blobs, and fallback stores might return strings or binary data
        if (kvClient._baseKv && typeof kvClient._baseKv.get === 'function') {
            // Vercel KV: Use get, it should return string or null
            const value = await kvClient._baseKv.get(`${VERCEL_KV_PREFIX}/${key}`);
            // Ensure it's a string or return undefined if null/not string
            return (typeof value === 'string') ? value : undefined;
        } else if (isNetlifyBlobs && kvClient._store && typeof kvClient._store.get === 'function') {
            // Netlify Blobs: Get as text
            const value = await kvClient._store.get(key, { type: "text" });
            return value === null ? undefined : value; // Already a string or null
        } else {
            // Fallback: Get potentially as bytes and decode
            const rawValue = await kv_get(key);
            if (rawValue === undefined || rawValue === null) {
                return undefined;
            }
            let textValue;
            if (rawValue instanceof Uint8Array) {
                textValue = new TextDecoder().decode(rawValue);
            } else if (typeof rawValue === 'string') {
                textValue = rawValue;
            } else { // Should not happen with kv_get logic, but handle defensively
                console.warn(`kv_get_text: KV fallback returned unexpected type for key ${key}:`, typeof rawValue);
                return undefined;
            }
            return textValue;
        }
    } catch (error) {
        // Log errors, especially if it's not a simple 'not found'
        if (error.message && error.message.includes('not found')) {
            console.debug(`KV get_text: Key ${key} not found.`);
            return undefined; // Indicate not found
        }
        console.error(`KV get_text error for ${key}:`, error);
        throw new Error(`Failed to get text for key ${key}: ${error.message}`);
    }
}

// Both Vercel KV and Netlify Blobs can handle binary data or JSON directly
// We'll trust the adapter to handle Uint8Array/JSON values appropriately
async function kv_set(key, value /* Uint8Array from Rust */) {
    try {
        const kvClient = await getKv();
        await kvClient.set(key, value);
    } catch (error) {
        console.error(`KV set error for ${key}:`, error);
        throw new Error(`Failed to set key ${key}: ${error.message}`);
    }
}

async function kv_set_text(key, value /* String from Rust */) {
    try {
        const kvClient = await getKv();
        // Pass the string value directly to the underlying store
        if (kvClient._baseKv && typeof kvClient._baseKv.set === 'function') {
            // Vercel KV: Use prefix and set string directly
            await kvClient._baseKv.set(`${VERCEL_KV_PREFIX}/${key}`, value);
        } else if (isNetlifyBlobs && kvClient._store && typeof kvClient._store.set === 'function') {
            // Netlify Blobs: Use set with string (implicitly handles encoding)
            await kvClient._store.set(key, value);
        }
        else {
            // Fallback: Use the kvClient.set which handles memory/byte conversion
            // Determine if the underlying kv.set expects string or Uint8Array
            if (kvClient.set === localStorageMap.set) { // Check if it's the in-memory fallback
                await kvClient.set(key, value); // In-memory stores string
            } else {
                // Assume other fallbacks might need bytes
                await kvClient.set(key, new TextEncoder().encode(value));
            }
        }
    } catch (error) {
        console.error(`KV set_text error for ${key}:`, error);
        throw new Error(`Failed to set text for key ${key}: ${error.message}`);
    }
}

async function kv_exists(key) {
    try {
        const kvClient = await getKv();
        const exists = await kvClient.exists(key);
        return exists > 0;
    } catch (error) {
        console.error(`KV exists error for ${key}:`, error);
        return false;
    }
}

async function kv_list(prefix) {
    try {
        const kvClient = await getKv();

        // If using Netlify Blobs, use its native list method with prefix support
        if (isNetlifyBlobs && kvClient._store) {
            const result = await kvClient._store.list({ prefix });
            return result.blobs.map(blob => blob.key);
        }

        // Otherwise, fall back to using scan with pattern matching
        let cursor = 0;
        const keys = [];
        let scanResult;

        do {
            // Use SCAN with MATCH to find keys with the given prefix
            scanResult = await kvClient.scan(cursor, {
                match: `${prefix}*`,
                count: 100 // Limit number of keys per scan
            });

            cursor = scanResult[0]; // Update cursor for next iteration
            const resultKeys = scanResult[1]; // Array of keys from this scan

            if (resultKeys && resultKeys.length > 0) {
                keys.push(...resultKeys);
            }
        } while (cursor !== '0'); // Continue until cursor becomes '0'

        return keys;
    } catch (error) {
        console.error(`KV list error for prefix ${prefix}:`, error);
        return [];
    }
}

async function kv_del(key) {
    try {
        const kvClient = await getKv();
        await kvClient.del(key);
    } catch (error) {
        console.error(`KV del error for ${key}:`, error);
    }
}

// Use global fetch available in Edge runtime
async function fetch_url(url) {
    try {
        const response = await fetch(url);
        return response;
    } catch (error) {
        console.error(`Fetch error for ${url}:`, error);
        throw error;
    }
}

// Helper to get status from Response
async function response_status(response /* Response object */) {
    // Add type check for robustness
    if (!(response instanceof Response)) {
        throw new Error("Input is not a Response object");
    }
    return response.status;
}

// Helper to get body as bytes (Uint8Array) from Response
async function response_bytes(response /* Response object */) {
    // Add type check for robustness
    if (!(response instanceof Response)) {
        throw new Error("Input is not a Response object");
    }
    try {
        const buffer = await response.arrayBuffer();
        return new Uint8Array(buffer);
    } catch (error) {
        console.error(`Error reading response body:`, error);
        throw error;
    }
}

// WASM-compatible fetch function that works in Node.js environment
async function wasm_fetch_with_request(url, options) {
    try {
        // In Node.js environment, use node-fetch or global fetch
        // This mimics the browser's fetch API for WASM
        let headers = {};

        // Extract headers from options if present
        if (options && options.headers) {
            const headerEntries = Object.entries(options.headers);
            for (const [key, value] of headerEntries) {
                headers[key] = value;
            }
        }

        // Use the method from options or default to GET
        const method = options && options.method ? options.method : 'GET';
        const body = options && options.body ? options.body : undefined;

        // Use either global fetch (Node.js 18+) or require node-fetch
        let fetchFunc = fetch;
        if (typeof fetch === 'undefined') {
            try {
                const nodeFetch = require('node-fetch');
                fetchFunc = nodeFetch;
            } catch (e) {
                console.error('Neither global fetch nor node-fetch is available:', e);
                throw new Error('No fetch implementation available');
            }
        }

        const response = await fetchFunc(url, {
            method,
            headers,
            body,
            // Add other options as needed
        });

        return response;
    } catch (error) {
        console.error(`WASM fetch error for ${url}:`, error);
        throw error;
    }
}

// Helper to get headers from Response as an object
async function response_headers(response) {
    if (!(response instanceof Response)) {
        throw new Error("Input is not a Response object");
    }

    const headers = {};
    for (const [key, value] of response.headers.entries()) {
        headers[key] = value;
    }

    return headers;
}

// Helper to get text from Response
async function response_text(response) {
    if (!(response instanceof Response)) {
        throw new Error("Input is not a Response object");
    }

    return await response.text();
}

function dummy() {
    return "dummy";
}

// --- Migration Placeholder --- 

/**
 * Migrates data from an old storage version to the current version.
 * This is a placeholder and needs to be implemented when a migration is required.
 * 
 * @param {number} oldVersion The version detected in storage.
 * @param {number} newVersion The current storage version defined in the code.
 */
async function migrateStorage(oldVersion, newVersion) {
    console.warn(`Storage migration needed from v${oldVersion} to v${newVersion}. Migration logic not implemented yet.`);
    // Example steps:
    // 1. Get access to the old version's store/client (e.g., using getStore(`...v${oldVersion}`))
    // 2. List keys from the old store.
    // 3. For each key/value:
    //    a. Read from old store.
    //    b. Transform data if necessary.
    //    c. Write to the *new* version's store (using the main `getKv()` which points to the new version).
    //    d. Optionally, delete from the old store after successful migration.
    // 4. Handle errors carefully.
    // 5. Update the storage version marker only after successful migration.

    // Placeholder implementation - does nothing currently
    await Promise.resolve(); // Simulate async operation
}

// Export all functions using CommonJS syntax
module.exports = {
    localStorageMap,
    getKv, // Expose getKv which now handles versioning internally
    kv_get,
    kv_set,
    kv_exists,
    kv_list,
    kv_del,
    fetch_url,
    response_status,
    response_bytes,
    wasm_fetch_with_request,
    response_headers,
    response_text,
    getenv,
    dummy,
    migrateStorage, // Expose migrate function if needed externally
    kv_get_text,
    kv_set_text,
}; 
import { NextRequest, NextResponse } from 'next/server';
import { loadWasmSingleton } from '@/lib/wasm';

/**
 * Handle subscription conversion requests
 * Converts upstream subscription URLs to various client-compatible formats
 */
export async function GET(request: NextRequest) {
    let wasmModule;
    try {
        wasmModule = await loadWasmSingleton('SubAPI');
    } catch (error) {
        console.error("Failed to load WASM module:", error);
        return NextResponse.json(
            {
                error: 'Failed to load WASM module',
                details: error instanceof Error ? error.message : String(error)
            },
            { status: 500 }
        );
    }

    // Construct a JSON query object from URL parameters
    const params: any = Object.fromEntries(request.nextUrl.searchParams);
    const requestHeaders = Object.fromEntries(request.headers.entries());
    params['request_headers'] = requestHeaders;

    // Normalize the 'url' parameter if it exists
    if (typeof params.url === 'string') {
        const urls = params.url.split(/\n|\|/).map((u: string) => u.trim()).filter(Boolean);
        params.url = urls.join('|');
    }

    if (!params.target) {
        return NextResponse.json(
            { error: 'Missing required parameter: target' },
            { status: 400 }
        );
    }

    console.log(`Sub API request: GET /api/sub with target=${params.target}`);

    try {
        // Convert parameters to a JSON string for the WASM function
        const queryJson = JSON.stringify(params);

        // Call the WASM function to process the subscription
        const responsePromise = wasmModule.sub_process_wasm(queryJson);

        // Wait for the Promise to resolve since sub_process_wasm returns a Promise
        const responseJsonString = await responsePromise;

        // Parse the response JSON
        const response = JSON.parse(responseJsonString);

        // Determine content type and headers from the response
        const contentType = response.content_type || 'text/plain';
        const headers = response.headers || {};

        // Create response with appropriate content type and headers
        const nextResponse = new NextResponse(response.content, {
            status: response.status_code || 200,
            headers: {
                'Content-Type': contentType,
                ...headers
            }
        });

        return nextResponse;
    } catch (error: any) {
        console.error(`Error processing subscription:`, error);
        const errorMessage = typeof error === 'string' ? error : (error.message || 'Unknown WASM error');

        return NextResponse.json(
            { error: 'Failed to process subscription', details: errorMessage },
            { status: 500 }
        );
    }
}

/**
 * Handle POST requests for subscription conversion
 * This allows clients to send subscription parameters in the request body
 */
export async function POST(request: NextRequest) {
    let wasmModule;
    try {
        wasmModule = await loadWasmSingleton('SubAPI');
    } catch (error) {
        console.error("Failed to load WASM module:", error);
        return NextResponse.json(
            {
                error: 'Failed to load WASM module',
                details: error instanceof Error ? error.message : String(error)
            },
            { status: 500 }
        );
    }

    try {
        // Parse request body as JSON
        const paramsJsonStr = await request.text();
        const params = JSON.parse(paramsJsonStr);

        // Normalize the 'url' parameter if it exists in the body
        if (typeof params.url === 'string') {
            const urls = params.url.split(/\n|\|/).map((u: string) => u.trim()).filter(Boolean);
            params.url = urls.join('|');
        }

        // Convert the potentially modified params back to JSON string
        const finalParamsJsonStr = JSON.stringify(params);

        // Call the WASM function to process the subscription
        const responsePromise = wasmModule.sub_process_wasm(finalParamsJsonStr);

        // Wait for the Promise to resolve
        const responseJsonString = await responsePromise;

        // Parse the response JSON
        const response = JSON.parse(responseJsonString);

        // Determine content type and headers from the response
        const contentType = response.content_type || 'text/plain';
        const headers = response.headers || {};

        // Create response with appropriate content type and headers
        const nextResponse = new NextResponse(response.content, {
            status: response.status_code || 200,
            headers: {
                'Content-Type': contentType,
                ...headers
            }
        });

        return nextResponse;
    } catch (error: any) {
        console.error(`Error processing subscription:`, error);
        const errorMessage = typeof error === 'string' ? error : (error.message || 'Unknown WASM error');

        return NextResponse.json(
            { error: 'Failed to process subscription', details: errorMessage },
            { status: 500 }
        );
    }
} 
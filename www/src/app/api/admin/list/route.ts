import { NextRequest, NextResponse } from 'next/server';
import { loadWasmSingleton } from '@/lib/wasm';

/**
 * Handle directory listing requests
 */
export async function GET(request: NextRequest) {
    let wasmModule;
    try {
        wasmModule = await loadWasmSingleton('Admin');
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

    // Get directory path from search params
    const path = request.nextUrl.searchParams.get('path') || '';
    const debug = request.nextUrl.searchParams.get('debug') === 'true';
    const shallow = request.nextUrl.searchParams.get('shallow') === 'true';

    console.log(`Admin API request: GET /api/admin/list?path=${path}${debug ? '&debug=true' : ''}${shallow ? '&shallow=true' : ''}`);

    try {
        console.log(`Listing directory: ${path}`);
        const entries = await wasmModule.list_directory(path);
        return NextResponse.json({
            path,
            entries
        });
    } catch (error: any) {
        console.error(`Error listing directory ${path}:`, error);
        const errorMessage = typeof error === 'string' ? error : (error.message || 'Unknown WASM error');

        if (errorMessage.includes('Not found')) {
            return NextResponse.json(
                { error: `Directory not found: ${path}`, details: errorMessage },
                { status: 404 }
            );
        } else {
            return NextResponse.json(
                { error: `Failed to list directory: ${path}`, details: errorMessage },
                { status: 500 }
            );
        }
    }
} 
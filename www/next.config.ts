import path from 'path';
import { fileURLToPath } from 'url';
import type { NextConfig } from "next";
import webpack from 'webpack';
import createNextIntlPlugin from 'next-intl/plugin';

const withNextIntl = createNextIntlPlugin();

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Detect environments
const isNetlify = process.env.NETLIFY === 'true' ||
  process.env.CONTEXT === 'production' ||
  process.env.NETLIFY_LOCAL === 'true' ||
  (process.env.DEPLOY_URL && process.env.DEPLOY_URL.includes('netlify'));

const isVercel = process.env.VERCEL === 'true';
const isDev = process.env.NODE_ENV === 'development';

// Log environment info
console.log('✅ Is Netlify environment:', isNetlify);
console.log('✅ Is Vercel environment:', isVercel);
console.log('✅ Is Development environment:', isDev);

const nextConfig: NextConfig = {
  reactStrictMode: true,
  // Allows importing wasm files from pkg directory
  // transpilePackages: ['subconverter-wasm'],

  // Using serverExternalPackages to tell Next.js to resolve the WASM module at runtime
  // This ensures proper WASM loading in server environments like Netlify
  serverExternalPackages: ['subconverter-wasm', '../pkg'],

  // Webpack config to support WASM
  webpack: (config, { isServer, dev }) => {
    console.log(`⚙️ Configuring webpack (isServer: ${isServer}, dev: ${dev})`);

    // Support for WebAssembly
    config.experiments = {
      ...config.experiments,
      asyncWebAssembly: true,
      layers: true,
      topLevelAwait: true,
    };

    // Configure WASM output location
    if (config.output) {
      // Ensure WASM is properly emitted to a predictable location
      if (isNetlify) {
        // For Netlify, use a completely predictable name and location
        config.output.webassemblyModuleFilename = isServer
          ? '../static/wasm/[modulehash].wasm'  // Server build
          : 'static/wasm/[modulehash].wasm';    // Client build
      } else {
        // For other environments
        config.output.webassemblyModuleFilename = isServer
          ? '../static/wasm/[modulehash].wasm'  // Server build 
          : 'static/wasm/[modulehash].wasm';    // Client build
      }
    }

    // Define environment variable to help with debugging WASM loading
    config.plugins = config.plugins || [];
    config.plugins.push(
      new webpack.DefinePlugin({
        'process.env.WASM_DEBUG': JSON.stringify(true),
        'process.env.DEPLOY_ENV': JSON.stringify(
          isNetlify ? 'netlify' : (isVercel ? 'vercel' : 'standard')
        ),
      })
    );

    // Make sure we don't interfere with the existing loaders
    return config;
  },
  async rewrites() {
    return [
      // Rewrite all API calls to the pages/api directory
      {
        source: '/api/:path*',
        destination: '/api/:path*',
      },
    ];
  },
  outputFileTracingIncludes: {
    '/api/': ['./node_modules/subconverter-wasm/**/*'],
  },
};

export default withNextIntl(nextConfig);

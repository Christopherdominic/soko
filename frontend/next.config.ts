import type { NextConfig } from "next";

// The browser only talks to the Next.js origin; /api/* is proxied to the Rust backend.
// This keeps session cookies same-origin and the backend off the public internet.
const backendUrl = process.env.SOKO_BACKEND_URL ?? "http://127.0.0.1:4000";

const nextConfig: NextConfig = {
  cacheComponents: true,
  partialPrefetching: true,
  turbopack: {
    rules: {
      "*.css": {
        loaders: ["@tailwindcss/turbopack"],
        as: "*.css",
      },
    },
  },
  async rewrites() {
    return [{ source: "/api/:path*", destination: `${backendUrl}/api/:path*` }];
  },
};

export default nextConfig;

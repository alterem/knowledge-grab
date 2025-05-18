import { loadEnv } from 'vite';
import path from 'path';
import vue from "@vitejs/plugin-vue";
import tailwindcss from '@tailwindcss/vite';
import obfuscator from 'rollup-plugin-obfuscator';

// https://vitejs.dev/config/
export default async (_command: any, mode: string) => {
  const env = loadEnv(mode, __dirname, 'ALTEREM_');
  const isProd = mode === 'production';
  
  return {
    envPrefix: "ALTEREM_",
    plugins: [vue(), tailwindcss()],
    server: {
      host: true,
      // open: true,
      port: env.ALTEREM_PORT,
      hmr: {
        overlay: true,
      },
    },
    resolve: {
      alias: {
        '@': path.resolve(__dirname, './src'),
      },
    },
    build: {
      rollupOptions: {
        plugins: [
          isProd && obfuscator({
            options: {
              compact: true,
              controlFlowFlattening: true,
              controlFlowFlatteningThreshold: 0.7,
              stringArray: true,
              stringArrayEncoding: ['rc4'],
              stringArrayThreshold: 0.8,
              rotateStringArray: true,
              shuffleStringArray: true,
              splitStrings: true,
              splitStringsChunkLength: 10,
              identifierNamesGenerator: 'hexadecimal',
              renameGlobals: false,
              deadCodeInjection: true,
              deadCodeInjectionThreshold: 0.4,
              debugProtection: false,
              disableConsoleOutput: true,
              selfDefending: true,
              transformObjectKeys: true,
              unicodeEscapeSequence: false
            }
          })
        ].filter(Boolean)
      }
    }
  }
};

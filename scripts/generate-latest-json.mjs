#!/usr/bin/env node
// 汇总各平台构建产物，生成 Tauri updater 的静态清单 latest.json。
// 用法: node scripts/generate-latest-json.mjs <artifactsDir> <tag> <owner/repo>
// 更新说明通过环境变量 RELEASE_NOTES 传入（release job 里已生成的 changelog）。

import { readdirSync, readFileSync, renameSync, writeFileSync } from 'node:fs';
import { basename, join } from 'node:path';

const [artifactsDir, tag, repo] = process.argv.slice(2);
if (!artifactsDir || !tag || !repo) {
  console.error('用法: node generate-latest-json.mjs <artifactsDir> <tag> <owner/repo>');
  process.exit(1);
}
const version = tag.replace(/^v/, '');

// artifact 目录名 → updater 平台 key 与该平台的 updater 安装包匹配规则
// （Windows 用 NSIS setup.exe，macOS 用 .app.tar.gz，Linux 仅 AppImage 支持自动更新）
const PLATFORMS = [
  { dir: 'windows-x64', key: 'windows-x86_64', pick: (f) => f.endsWith('-setup.exe') },
  { dir: 'windows-arm64', key: 'windows-aarch64', pick: (f) => f.endsWith('-setup.exe') },
  { dir: 'linux-x64', key: 'linux-x86_64', pick: (f) => f.endsWith('.AppImage') },
  { dir: 'macos-arm64', key: 'darwin-aarch64', pick: (f) => f.endsWith('.app.tar.gz'), archSuffix: 'aarch64' },
  { dir: 'macos-x64', key: 'darwin-x86_64', pick: (f) => f.endsWith('.app.tar.gz'), archSuffix: 'x64' },
];

function walk(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    return entry.isDirectory() ? walk(path) : [path];
  });
}

const platforms = {};
for (const { dir, key, pick, archSuffix } of PLATFORMS) {
  let files;
  try {
    files = walk(join(artifactsDir, dir));
  } catch {
    console.warn(`::warning::缺少产物目录 ${dir}，latest.json 将不包含 ${key}`);
    continue;
  }

  let asset = files.find((file) => pick(basename(file)));
  if (!asset) {
    console.warn(`::warning::${dir} 中未找到 updater 安装包，跳过 ${key}`);
    continue;
  }

  // 两个 macOS 架构的 .app.tar.gz 同名，上传 release 前重命名避免资源名冲突
  if (archSuffix && !basename(asset).includes(archSuffix)) {
    const renamed = asset.replace(/\.app\.tar\.gz$/, `_${version}_${archSuffix}.app.tar.gz`);
    renameSync(asset, renamed);
    try {
      renameSync(`${asset}.sig`, `${renamed}.sig`);
    } catch {
      // .sig 缺失时由下方签名读取统一报错
    }
    asset = renamed;
  }

  let signature;
  try {
    signature = readFileSync(`${asset}.sig`, 'utf8').trim();
  } catch {
    console.error(
      `::error::${basename(asset)} 缺少签名文件 .sig（检查 TAURI_SIGNING_PRIVATE_KEY secrets 与 createUpdaterArtifacts 配置）`
    );
    process.exit(1);
  }

  platforms[key] = {
    signature,
    url: `https://github.com/${repo}/releases/download/${tag}/${encodeURIComponent(basename(asset))}`,
  };
}

if (Object.keys(platforms).length === 0) {
  console.error('::error::没有任何平台的 updater 产物，latest.json 生成失败');
  process.exit(1);
}

const manifest = {
  version,
  notes: (process.env.RELEASE_NOTES || '').trim() || `Release ${tag}`,
  pub_date: new Date().toISOString(),
  platforms,
};

writeFileSync(join(artifactsDir, 'latest.json'), JSON.stringify(manifest, null, 2) + '\n');
console.log(`已生成 latest.json，包含平台: ${Object.keys(platforms).join(', ')}`);

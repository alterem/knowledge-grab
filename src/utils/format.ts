// 大数字紧凑显示：1.2万 / 3.5亿，保持列表行宽度稳定
export function formatCount(value: number): string {
  if (!Number.isFinite(value) || value < 0) return '0';
  if (value < 10000) return value.toLocaleString('en-US');
  if (value < 100000000) {
    const wan = value / 10000;
    return `${wan >= 100 ? Math.round(wan) : parseFloat(wan.toFixed(1))}万`;
  }
  const yi = value / 100000000;
  return `${yi >= 100 ? Math.round(yi) : parseFloat(yi.toFixed(1))}亿`;
}

// 字节数人类可读显示：825 KB / 1.4 MB / 2.05 GB
export function formatBytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let size = value;
  let unit = 0;
  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024;
    unit += 1;
  }
  return `${size >= 100 || unit === 0 ? Math.round(size) : parseFloat(size.toFixed(unit >= 3 ? 2 : 1))} ${units[unit]}`;
}

// 下载速度显示：formatBytes + "/s"
export function formatSpeed(bytesPerSecond: number): string {
  return `${formatBytes(bytesPerSecond)}/s`;
}

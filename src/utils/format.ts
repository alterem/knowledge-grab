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

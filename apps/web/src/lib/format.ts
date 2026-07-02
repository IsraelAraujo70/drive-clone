const UNITS = ["B", "KB", "MB", "GB", "TB"];

export function formatBytes(bytes: number): string {
  let value = Math.max(bytes, 0);
  let index = 0;
  while (value >= 1024 && index < UNITS.length - 1) {
    value /= 1024;
    index += 1;
  }
  const text = index === 0 ? String(value) : value.toFixed(1).replace(/\.0$/, "");
  return `${text} ${UNITS[index]}`;
}

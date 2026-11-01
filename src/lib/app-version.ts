/** 只转换个人版数字修订的展示, IPC/更新排序仍保留原始 SemVer. */
export function displayAppVersion(version: string): string {
  return version.replace(/^(\d+\.\d+\.\d+)\+([1-9]\d*)$/, "$1.$2");
}

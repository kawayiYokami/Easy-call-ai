// @lucide/vue v1.16+ 导出了 LUCIDE_CONTEXT 但未在类型声明中暴露
// 通过 esm 路径直接导入以绕过 TypeScript 类型检查
// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore -- LUCIDE_CONTEXT 运行时存在但类型未导出
export { LUCIDE_CONTEXT } from "@lucide/vue";

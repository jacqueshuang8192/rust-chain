# 修复移动端 vue-tsc 依赖入口缺失

## Goal

修复移动端类型检查在 pnpm 安装环境下因 `vue-tsc@2.0.0` 发布包缺少
`index.js` 而无法启动的问题，并让依赖声明明确避开该损坏版本。

## What I already know

- `mobile/package.json` 当前声明 `vue-tsc: ^2.0.0`。
- 用户错误来自 `mobile/node_modules/.pnpm/vue-tsc@2.0.0_typescript@5.4.2`。
- 该版本的 `bin/vue-tsc.js` 执行 `require('../index.js')`，但发布包中不存在该文件。
- `mobile/package-lock.json` 已解析到可用的 `vue-tsc@2.2.12`，但根依赖声明仍是 `^2.0.0`。
- 当前工作区的 `mobile/pnpm-lock.yaml`、`mobile/pnpm-workspace.yaml` 是未跟踪文件，不纳入本次修改；仓库既有移动端锁文件是 `mobile/package-lock.json`。

## Assumptions

- 保持现有 Vue、TypeScript、Vite 和构建脚本不变，只修复类型检查工具的最低版本约束。
- 不在仓库中提交 `node_modules` 或本地生成的 pnpm 文件。

## Requirements

- 将 `vue-tsc` 最低版本提升到包含根入口文件的 `^2.2.12`。
- 同步 `mobile/package-lock.json` 的根依赖声明，保留已锁定的 `2.2.12` 包内容。
- 显式声明测试类型检查直接使用的 `@vue/compiler-sfc`，兼容 pnpm 的严格依赖隔离。
- 清理并重新安装依赖后，`vue-tsc --noEmit` 可以正常启动并完成类型检查。

## Acceptance Criteria

- [x] `mobile/package.json` 和 `mobile/package-lock.json` 不再允许解析到 `vue-tsc@2.0.0`。
- [x] 干净安装后的 `vue-tsc --noEmit` 不再报 `Cannot find module '../index.js'`。
- [x] 移动端现有测试类型检查和发布门禁中与依赖相关的步骤通过。

## Definition of Done

- 更新依赖声明与锁文件。
- 执行移动端类型检查和至少一组移动端质量验证。
- 更新 `docs/superpowers/PROGRESS.md`。
- 完成 `git diff --check`，提交本地改动。

## Out of Scope

- 不修改移动端业务代码、Vue 组件或 API。
- 不提交本地 pnpm workspace/lockfile，除非仓库原有包管理策略明确要求。

## Technical Notes

- 相关文件：`mobile/package.json`、`mobile/package-lock.json`。
- `vue-tsc@2.2.12` 的发布包包含 `index.js`，并兼容 TypeScript `>=5.0.0`。

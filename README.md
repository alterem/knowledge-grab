# knowledge-grab

![Build Status](https://github.com/alterem/knowledge-grab/actions/workflows/build.yml/badge.svg) ![GitHub release (latest by date)](https://img.shields.io/github/v/release/alterem/knowledge-grab)

## 项目简介

`knowledge-grab` 是一个基于 [Tauri](https://tauri.app/) 和 [Vue 3](https://vuejs.org/) 构建的桌面应用程序，方便用户从 [国家中小学智慧教育平台 (basic.smartedu.cn)](https://basic.smartedu.cn/) 下载各类教育资源。


## 模板

> 👏 欢迎 Starred & Use this template 

#### Vue + Naive UI

https://github.com/alterem/tauri-vue-template/tree/naiveui

#### Vue + Element Plus

https://github.com/alterem/tauri-vue-template

#### React + Ant Design

https://github.com/alterem/tauri-react-template

## 技术栈

- **框架**: Vue 3 (使用 Composition API)
- **构建工具**: Vite
- **桌面应用框架**: Tauri
- **包管理器**: pnpm

## 功能

- 支持从国家中小学智慧教育平台下载特定教育资源。
- 支持批量下载功能。
- 支持按分类下载。

## 一些截图

![Screenshot of the main window](https://raw.githubusercontent.com/alterem/picFB/master/uPic/2026/07/16/sj9yYp.png)

![Screenshot of the main window](https://raw.githubusercontent.com/alterem/picFB/master/uPic/2026/07/16/ShFxKs.png)

![Screenshot of the cover preview](https://raw.githubusercontent.com/alterem/picFB/master/uPic/2026/07/16/r68sKX.png)

![Screenshot of the setting window](https://raw.githubusercontent.com/alterem/picFB/master/uPic/2026/07/16/1G94ED.png)


## 环境要求

- [Node.js](https://nodejs.org/) (推荐 LTS 版本)
- [Rust](https://www.rust-lang.org/tools/install) (Tauri 框架需要)
- 构建 Tauri 应用所需的其他依赖项 (详见 [Tauri 官方文档 - Prerequisites](https://v2.tauri.app/start/prerequisites))

## 启动项目 (开发模式)

1.  克隆仓库到本地：
    ```bash
    git clone https://github.com/alterem/knowledge-grab
    cd knowledge-grab
    ```
2.  安装项目依赖：
    ```bash
    pnpm install
    ```
3.  启动 Tauri 开发模式。这会同时启动前端开发服务器和 Rust 后端：
    ```bash
    pnpm tauri dev
    ```
    应用程序窗口会打开，前端代码修改会实时反映。

## 打包项目 (构建发布版本)

1.  确保你已经安装了所有依赖 (见上一步)。
2.  运行 Tauri 构建命令：
    ```bash
    pnpm tauri build
    ```
    这个命令会构建前端项目并将 Rust 后端编译成可执行文件，生成对应操作系统的安装包或可执行文件。构建好的文件通常在 `src-tauri/target/release/bundle/` 目录下。

## 常见问题 (FAQ)

**Q: 在 macOS 上下载的应用无法直接打开，提示“无法验证开发者”或类似错误怎么办？**

A: 这是 macOS 的 Gatekeeper 安全机制导致的。应用未经过 Apple 的开发者认证，首次打开可能会被阻止。可以在终端执行以下命令来允许应用运行：

```bash
xattr -rd com.apple.quarantine /Applications/KnowledgeGrab.app
```

请根据实际安装路径修改 `/Applications/KnowledgeGrab.app`。执行此命令后，应该就能正常打开应用了。

**Q: 下载时出现 403 / 401 错误，或提示需要 Access Token 怎么办？**

A: 平台已改版，电子课本资源迁移到了需要登录鉴权的服务器，下载教材必须提供有效的 Access Token（登录凭据）。请进入应用「设置」页：

- 点击「登录平台自动获取」，在弹出的官方登录窗口完成登录，应用会自动抓取并保存令牌；
- 或点击「查看手动获取步骤」，按提示在浏览器控制台运行脚本，复制令牌后手动填入。

令牌仅保存在本机，约一周过期，失效后按同样方式重新获取即可。

## 参与贡献

欢迎提交 Issue 或 Pull Request。

## 🏢 赞助

## 开源许可

本项目采用 [MIT 许可协议](LICENSE)。


## Stargazers over time

![Stargazers over time](https://starchart.cc/alterem/knowledge-grab.svg?variant=adaptive)
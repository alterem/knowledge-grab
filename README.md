# knowledge-grab

![Build Status](https://github.com/alterem/knowledge-grab/actions/workflows/build.yml/badge.svg) ![GitHub release (latest by date)](https://img.shields.io/github/v/release/alterem/knowledge-grab)

## 项目简介

`knowledge-grab` 是一个基于 [Tauri](https://tauri.app/) 和 [Vue 3](https://vuejs.org/) 构建的桌面应用程序，旨在帮助用户方便地从 [国家中小学智慧教育平台 (basic.smartedu.cn)](https://basic.smartedu.cn/) 下载各类教育资源。

## 技术栈

- **框架**: Vue 3 (使用 Composition API)
- **构建工具**: Vite
- **桌面应用框架**: Tauri
- **包管理器**: pnpm

## 功能

- 从国家中小学智慧教育平台下载指定资源。
- 支持批量下载
- 分类下载

## 一些截图

![Screenshot of the main window](https://raw.githubusercontent.com/alterem/picFB/master/uPic/2025/05/18/T0jSWE.png)

![Screenshot of the cover preview](https://raw.githubusercontent.com/alterem/picFB/master/uPic/2025/05/18/MAA9U0.png)

![Screenshot of the setting window](https://raw.githubusercontent.com/alterem/picFB/master/uPic/2025/05/18/aRMaMr.png)


## 环境要求

- [Node.js](https://nodejs.org/) (推荐 LTS 版本)
- [Rust](https://www.rust-lang.org/tools/install) (由 Tauri 框架需要)
- 构建 Tauri 应用所需的其他依赖 (详见 [Tauri 官方文档 - Prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites))

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
    应用程序窗口将会打开，并且前端代码的修改会实时反映。

## 打包项目 (构建发布版本)

1.  确保你已经安装了所有依赖 (见上一步)。
2.  运行 Tauri 构建命令：
    ```bash
    pnpm tauri build
    ```
    这个命令会构建前端项目并将 Rust 后端编译成可执行文件，最终生成对应操作系统的安装包或可执行文件。构建好的文件通常位于 `src-tauri/target/release/bundle/` 目录下。

## 参与贡献

欢迎提交 Issue 或 Pull Request。

## 开源许可

本项目采用 [MIT 许可协议](LICENSE)。
